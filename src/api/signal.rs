use std::collections::VecDeque;

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    signal_service_server, LayoutRequest, LayoutResponse, OutputConnectRequest,
    OutputConnectResponse, StreamControl, WindowPointerEnterRequest, WindowPointerEnterResponse,
    WindowPointerLeaveRequest, WindowPointerLeaveResponse,
};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tonic::{Request, Response, Status, Streaming};

use crate::state::State;

use super::{run_bidirectional_streaming, ResponseStream, StateFnSender};

#[derive(Debug, Default)]
pub struct SignalState {
    pub output_connect: SignalData<OutputConnectResponse, VecDeque<OutputConnectResponse>>,
    pub layout: SignalData<LayoutResponse, VecDeque<LayoutResponse>>,
    pub window_pointer_enter:
        SignalData<WindowPointerEnterResponse, VecDeque<WindowPointerEnterResponse>>,
    pub window_pointer_leave:
        SignalData<WindowPointerLeaveResponse, VecDeque<WindowPointerLeaveResponse>>,
}

impl SignalState {
    pub fn clear(&mut self) {
        self.output_connect.disconnect();
        self.layout.disconnect();
        self.window_pointer_enter.disconnect();
        self.window_pointer_leave.disconnect();
    }
}

#[derive(Debug, Default)]
#[allow(private_bounds)]
pub struct SignalData<T, B: SignalBuffer<T>> {
    sender: Option<UnboundedSender<Result<T, Status>>>,
    join_handle: Option<JoinHandle<()>>,
    ready: bool,
    buffer: B,
}

/// A trait that denotes different types of containers that can be used to buffer signals.
trait SignalBuffer<T>: Default {
    /// Get the next signal from this buffer.
    fn next(&mut self) -> Option<T>;
}

impl<T> SignalBuffer<T> for VecDeque<T> {
    fn next(&mut self) -> Option<T> {
        self.pop_front()
    }
}

impl<T> SignalBuffer<T> for Option<T> {
    fn next(&mut self) -> Option<T> {
        self.take()
    }
}

#[allow(private_bounds)]
impl<T, B: SignalBuffer<T>> SignalData<T, B> {
    /// Attempt to send a signal.
    ///
    /// If the client is ready to accept more of this signal, it will be sent immediately.
    /// Otherwise, the signal will remain stored in the underlying buffer until the client is ready.
    ///
    /// Use `with_buffer` to populate and manipulate the buffer with the data you want.
    pub fn signal(&mut self, with_buffer: impl FnOnce(&mut B)) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        with_buffer(&mut self.buffer);

        if self.ready {
            if let Some(data) = self.buffer.next() {
                sender.send(Ok(data)).expect("failed to send signal");
                self.ready = false;
            }
        }
    }

    pub fn connect(
        &mut self,
        sender: UnboundedSender<Result<T, Status>>,
        join_handle: JoinHandle<()>,
    ) {
        self.sender.replace(sender);
        if let Some(handle) = self.join_handle.replace(join_handle) {
            handle.abort();
        }
    }

    fn disconnect(&mut self) {
        self.sender.take();
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
        self.ready = false;
        self.buffer = B::default();
    }

    /// Mark this signal as ready to send.
    ///
    /// If there are signals already in the buffer, they will be sent.
    fn ready(&mut self) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        if let Some(data) = self.buffer.next() {
            sender.send(Ok(data)).expect("failed to send signal");
            self.ready = false;
        } else {
            self.ready = true;
        }
    }
}

trait SignalRequest {
    fn control(&self) -> StreamControl;
}

macro_rules! impl_signal_request {
    ( $( $request:ident ),* ) => {
        $(
            impl SignalRequest for $request {
                fn control(&self) -> StreamControl {
                    self.control()
                }
            }
        )*
    };
}

impl_signal_request!(
    OutputConnectRequest,
    LayoutRequest,
    WindowPointerEnterRequest,
    WindowPointerLeaveRequest
);

fn start_signal_stream<I, O, B, F>(
    sender: StateFnSender,
    in_stream: Streaming<I>,
    with_signal_buffer: F,
) -> Result<Response<ResponseStream<O>>, Status>
where
    I: SignalRequest + std::fmt::Debug + Send + 'static,
    O: Send + 'static,
    B: SignalBuffer<O>,
    F: Fn(&mut State) -> &mut SignalData<O, B> + Clone + Send + 'static,
{
    let with_signal_buffer_clone = with_signal_buffer.clone();

    run_bidirectional_streaming(
        sender,
        in_stream,
        move |state, request| {
            let request = match request {
                Ok(request) => request,
                Err(status) => {
                    tracing::error!("Error in output_connect signal in stream: {status}");
                    return;
                }
            };

            tracing::debug!("Got {request:?} from client stream");

            let signal = with_signal_buffer(state);
            match request.control() {
                StreamControl::Ready => signal.ready(),
                StreamControl::Disconnect => signal.disconnect(),
                StreamControl::Unspecified => tracing::warn!("Received unspecified stream control"),
            }
        },
        move |state, sender, join_handle| {
            let signal = with_signal_buffer_clone(state);
            signal.connect(sender, join_handle);
        },
    )
}

pub struct SignalService {
    sender: StateFnSender,
}

impl SignalService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl signal_service_server::SignalService for SignalService {
    type OutputConnectStream = ResponseStream<OutputConnectResponse>;
    type LayoutStream = ResponseStream<LayoutResponse>;
    type WindowPointerEnterStream = ResponseStream<WindowPointerEnterResponse>;
    type WindowPointerLeaveStream = ResponseStream<WindowPointerLeaveResponse>;

    async fn output_connect(
        &self,
        request: Request<Streaming<OutputConnectRequest>>,
    ) -> Result<Response<Self::OutputConnectStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.signal_state.output_connect
        })
    }

    async fn layout(
        &self,
        request: Request<Streaming<LayoutRequest>>,
    ) -> Result<Response<Self::LayoutStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.signal_state.layout
        })
    }

    async fn window_pointer_enter(
        &self,
        request: Request<Streaming<WindowPointerEnterRequest>>,
    ) -> Result<Response<Self::WindowPointerEnterStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.signal_state.window_pointer_enter
        })
    }

    async fn window_pointer_leave(
        &self,
        request: Request<Streaming<WindowPointerLeaveRequest>>,
    ) -> Result<Response<Self::WindowPointerLeaveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.signal_state.window_pointer_leave
        })
    }
}
