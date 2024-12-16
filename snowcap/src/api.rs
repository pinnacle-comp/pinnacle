pub mod input;

use std::{num::NonZeroU32, pin::Pin};

use futures::Stream;
use smithay_client_toolkit::{reexports::calloop, shell::wlr_layer};
use snowcap_api_defs::snowcap::layer::{
    self,
    v0alpha1::{layer_service_server, CloseRequest, NewLayerRequest, NewLayerResponse},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tonic::{Request, Response, Status};
use tracing::warn;

use crate::{
    layer::{ExclusiveZone, SnowcapLayer},
    state::State,
    widget::widget_def_to_fn,
};

async fn run_unary_no_response<F>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<()>, Status>
where
    F: FnOnce(&mut State) + Send + 'static,
{
    fn_sender
        .send(Box::new(with_state))
        .map_err(|_| Status::internal("failed to execute request"))?;

    Ok(Response::new(()))
}

async fn run_unary<F, T>(fn_sender: &StateFnSender, with_state: F) -> Result<Response<T>, Status>
where
    F: FnOnce(&mut State) -> Result<T, Status> + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel::<Result<T, Status>>();

    let f = Box::new(|state: &mut State| {
        // TODO: find a way to handle this error
        if sender.send(with_state(state)).is_err() {
            warn!("failed to send result of API call to config; receiver already dropped");
        }
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let ret = receiver.await;

    match ret {
        Ok(it) => Ok(Response::new(it?)),
        Err(err) => Err(Status::internal(format!(
            "failed to transfer response for transport to client: {err}"
        ))),
    }
}

fn run_server_streaming<F, T>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<ResponseStream<T>>, Status>
where
    F: FnOnce(&mut State, UnboundedSender<Result<T, Status>>) + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<Result<T, Status>>();

    let f = Box::new(|state: &mut State| {
        with_state(state, sender);
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream)))
}

type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

pub struct SnowcapService {
    _sender: StateFnSender,
}

impl SnowcapService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { _sender: sender }
    }
}

pub struct LayerService {
    sender: StateFnSender,
}

impl LayerService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl layer_service_server::LayerService for LayerService {
    async fn new_layer(
        &self,
        request: Request<NewLayerRequest>,
    ) -> Result<Response<NewLayerResponse>, Status> {
        let request = request.into_inner();

        let anchor = request.anchor();
        let exclusive_zone = request.exclusive_zone();
        let keyboard_interactivity = request.keyboard_interactivity();
        let layer = request.layer();

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let width = request.width.unwrap_or(600);
        let height = request.height.unwrap_or(480);

        let anchor = match anchor {
            layer::v0alpha1::Anchor::Unspecified => wlr_layer::Anchor::empty(),
            layer::v0alpha1::Anchor::Top => wlr_layer::Anchor::TOP,
            layer::v0alpha1::Anchor::Bottom => wlr_layer::Anchor::BOTTOM,
            layer::v0alpha1::Anchor::Left => wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::Right => wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::TopLeft => wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::TopRight => wlr_layer::Anchor::TOP | wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::BottomLeft => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::LEFT
            }
            layer::v0alpha1::Anchor::BottomRight => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::RIGHT
            }
        };
        let exclusive_zone = match exclusive_zone {
            0 => ExclusiveZone::Respect,
            x if x.is_positive() => ExclusiveZone::Exclusive(NonZeroU32::new(x as u32).unwrap()),
            _ => ExclusiveZone::Ignore,
        };

        let keyboard_interactivity = match keyboard_interactivity {
            layer::v0alpha1::KeyboardInteractivity::Unspecified
            | layer::v0alpha1::KeyboardInteractivity::None => {
                wlr_layer::KeyboardInteractivity::None
            }
            layer::v0alpha1::KeyboardInteractivity::OnDemand => {
                wlr_layer::KeyboardInteractivity::OnDemand
            }
            layer::v0alpha1::KeyboardInteractivity::Exclusive => {
                wlr_layer::KeyboardInteractivity::Exclusive
            }
        };

        let layer = match layer {
            layer::v0alpha1::Layer::Unspecified => wlr_layer::Layer::Top,
            layer::v0alpha1::Layer::Background => wlr_layer::Layer::Background,
            layer::v0alpha1::Layer::Bottom => wlr_layer::Layer::Bottom,
            layer::v0alpha1::Layer::Top => wlr_layer::Layer::Top,
            layer::v0alpha1::Layer::Overlay => wlr_layer::Layer::Overlay,
        };

        run_unary(&self.sender, move |state| {
            let Some((f, states)) = widget_def_to_fn(widget_def) else {
                return Err(Status::invalid_argument("widget def was null"));
            };

            let layer = SnowcapLayer::new(
                state,
                width,
                height,
                layer,
                anchor,
                exclusive_zone,
                keyboard_interactivity,
                crate::widget::SnowcapWidgetProgram {
                    widgets: f,
                    widget_state: states,
                },
            );

            let ret = Ok(NewLayerResponse {
                layer_id: Some(layer.widget_id.into_inner()),
            });

            state.layers.push(layer);

            ret
        })
        .await
    }

    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();

        let Some(id) = request.layer_id else {
            return Err(Status::invalid_argument("layer id was null"));
        };

        run_unary_no_response(&self.sender, move |state| {
            state
                .layers
                .retain(|sn_layer| sn_layer.widget_id.into_inner() != id);
        })
        .await
    }
}
