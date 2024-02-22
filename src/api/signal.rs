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
    pub output_connect: SignalData<OutputConnectResponse>,
    pub layout: SignalData<LayoutResponse>,
    pub window_pointer_enter: SignalData<WindowPointerEnterResponse>,
    pub window_pointer_leave: SignalData<WindowPointerLeaveResponse>,
}

#[derive(Debug, Default)]
pub struct SignalData<T> {
    sender: Option<UnboundedSender<Result<T, Status>>>,
    join_handle: Option<JoinHandle<()>>,
    ready: bool,
    value: Option<T>,
}

impl<T> SignalData<T> {
    pub fn signal(&mut self, with_data: impl FnOnce(Option<T>) -> T) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        if self.ready {
            sender
                .send(Ok(with_data(self.value.take())))
                .expect("failed to send signal");
            self.ready = false;
        } else {
            self.value = Some(with_data(self.value.take()));
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
        self.value.take();
    }

    fn ready(&mut self) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        if let Some(value) = self.value.take() {
            sender.send(Ok(value)).expect("failed to send signal");
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

fn start_signal_stream<I: SignalRequest + std::fmt::Debug, O>(
    sender: StateFnSender,
    in_stream: Streaming<I>,
    signal: impl Fn(&mut State) -> &mut SignalData<O> + Clone + Send + 'static,
) -> Result<Response<ResponseStream<O>>, Status>
where
    I: Send + 'static,
    O: Send + 'static,
{
    let signal_clone = signal.clone();

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

            tracing::info!("GOT {request:?} FROM CLIENT STREAM");

            let signal = signal(state);
            match request.control() {
                StreamControl::Ready => signal.ready(),
                StreamControl::Disconnect => signal.disconnect(),
                StreamControl::Unspecified => tracing::warn!("Received unspecified stream control"),
            }
        },
        move |state, sender, join_handle| {
            let signal = signal_clone(state);
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
