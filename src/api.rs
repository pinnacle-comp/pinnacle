pub mod input;
pub mod layout;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod render;
pub mod signal;
pub mod tag;
pub mod window;

use std::pin::Pin;

use smithay::reexports::calloop;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Response, Status, Streaming};
use tracing::{debug, warn};

use crate::state::State;

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
pub type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;
pub type TonicResult<T> = Result<Response<T>, Status>;

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

    let res = receiver.await.map_err(|err| {
        Status::internal(format!(
            "failed to transfer response for transport to client: {err}"
        ))
    });

    match res {
        Ok(res) => res.map(Response::new),
        Err(err) => Err(err),
    }
}

async fn run_server_streaming<F, T>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<ResponseStream<T>>, Status>
where
    F: FnOnce(&mut State, UnboundedSender<Result<T, Status>>) -> Result<(), Status>
        + Send
        + 'static,
    T: Send + 'static,
{
    let (msg_send, msg_recv) = tokio::sync::oneshot::channel::<Result<(), Status>>();
    let (sender, receiver) = unbounded_channel::<Result<T, Status>>();

    let f = Box::new(|state: &mut State| {
        if msg_send.send(with_state(state, sender)).is_err() {
            warn!("failed to send result of API call to config; receiver already dropped");
        }
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let res = msg_recv.await.map_err(|err| {
        Status::internal(format!(
            "failed to transfer response for transport to client: {err}"
        ))
    });

    let res = match res {
        Ok(res) => res.map(move |()| {
            Response::new(
                Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(
                    receiver,
                )) as _,
            )
        }),
        Err(err) => return Err(err),
    };

    res
}

/// Begin a bidirectional grpc stream.
///
/// # Parameters
/// - `fn_sender`: The function sender
/// - `in_stream`: The incoming client stream
/// - `on_client_request`: A callback that will be run with every received request.
/// - `with_out_stream_and_in_stream_join_handle`:
///     Do something with the outbound server-to-client stream.
///     This also receives the join handle for the tokio task listening to
///     the incoming client-to-server stream.
fn run_bidirectional_streaming<F1, F2, I, O>(
    fn_sender: StateFnSender,
    mut in_stream: Streaming<I>,
    on_client_request: F1,
    with_out_stream_and_in_stream_join_handle: F2,
) -> Result<Response<ResponseStream<O>>, Status>
where
    F1: Fn(&mut State, I) + Clone + Send + 'static,
    F2: FnOnce(&mut State, UnboundedSender<Result<O, Status>>, JoinHandle<()>) + Send + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    let (sender, receiver) = unbounded_channel::<Result<O, Status>>();

    let fn_sender_clone = fn_sender.clone();

    let with_in_stream = async move {
        while let Some(request) = in_stream.next().await {
            match request {
                Ok(request) => {
                    let on_client_request = on_client_request.clone();
                    // TODO: handle error
                    let _ = fn_sender_clone.send(Box::new(move |state: &mut State| {
                        on_client_request(state, request);
                    }));
                }
                Err(err) => {
                    debug!("bidirectional stream error: {err}");
                    break;
                }
            }
        }
    };

    let join_handle = tokio::spawn(with_in_stream);

    let with_out_stream_and_in_stream_join_handle = Box::new(|state: &mut State| {
        with_out_stream_and_in_stream_join_handle(state, sender, join_handle);
    });

    fn_sender
        .send(with_out_stream_and_in_stream_join_handle)
        .map_err(|_| Status::internal("failed to execute request"))?;

    let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
    Ok(Response::new(Box::pin(receiver_stream)))
}
