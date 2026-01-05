pub mod decoration;
pub mod input;
pub mod layer;
pub mod operation;
pub mod popup;
pub mod widget;

use std::pin::Pin;

use futures::Stream;
use smithay_client_toolkit::reexports::calloop;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio_stream::StreamExt;
use tonic::{Response, Status};
use tracing::warn;

use crate::state::State;

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

fn run_server_streaming_mapped<F, T, FM, O>(
    fn_sender: &StateFnSender,
    with_state: F,
    map: FM,
) -> Result<Response<ResponseStream<O>>, Status>
where
    F: FnOnce(&mut State, UnboundedSender<T>) + Send + 'static,
    T: Send + 'static,
    FM: Fn(T) -> Result<O, Status> + Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<T>();

    let f = Box::new(|state: &mut State| {
        with_state(state, sender);
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream.map(map))))
}

fn run_server_streaming<F, T>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<ResponseStream<T>>, Status>
where
    F: FnOnce(&mut State, UnboundedSender<Result<T, Status>>) + Send + 'static,
    T: Send + 'static,
{
    run_server_streaming_mapped(fn_sender, with_state, std::convert::identity)
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
