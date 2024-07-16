use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicU32, Ordering},
};

use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    signal_service_server, OutputConnectRequest, OutputConnectResponse, OutputDisconnectRequest,
    OutputDisconnectResponse, OutputMoveRequest, OutputMoveResponse, OutputResizeRequest,
    OutputResizeResponse, SignalRequest, StreamControl, TagActiveRequest, TagActiveResponse,
    WindowPointerEnterRequest, WindowPointerEnterResponse, WindowPointerLeaveRequest,
    WindowPointerLeaveResponse,
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, warn};

use crate::state::State;

use super::{run_bidirectional_streaming, ResponseStream, StateFnSender};

#[derive(Debug, Default)]
pub struct SignalState {
    // Output
    pub output_connect: SignalData<OutputConnectResponse, VecDeque<OutputConnectResponse>>,
    pub output_disconnect: SignalData<OutputDisconnectResponse, VecDeque<OutputDisconnectResponse>>,
    pub output_resize: SignalData<OutputResizeResponse, VecDeque<OutputResizeResponse>>,
    pub output_move: SignalData<OutputMoveResponse, VecDeque<OutputMoveResponse>>,

    // Window
    pub window_pointer_enter:
        SignalData<WindowPointerEnterResponse, VecDeque<WindowPointerEnterResponse>>,
    pub window_pointer_leave:
        SignalData<WindowPointerLeaveResponse, VecDeque<WindowPointerLeaveResponse>>,

    // Tag
    pub tag_active: SignalData<TagActiveResponse, VecDeque<TagActiveResponse>>,
}

impl SignalState {
    pub fn clear(&mut self) {
        self.output_connect.instances.clear();
        self.output_disconnect.instances.clear();
        self.output_resize.instances.clear();
        self.output_move.instances.clear();
        self.window_pointer_enter.instances.clear();
        self.window_pointer_leave.instances.clear();
    }
}

#[derive(Debug, Default)]
#[allow(private_bounds)]
pub struct SignalData<T, B: SignalBuffer<T>> {
    instances: HashMap<ClientSignalId, SignalInstance<T, B>>,
}

#[derive(Debug)]
struct SignalInstance<T, B: SignalBuffer<T>> {
    sender: UnboundedSender<Result<T, Status>>,
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

type ClientSignalId = u32;

static CLIENT_SIGNAL_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[allow(private_bounds)]
impl<T, B: SignalBuffer<T>> SignalData<T, B> {
    /// Attempt to send a signal.
    ///
    /// If the client is ready to accept more of this signal, it will be sent immediately.
    /// Otherwise, the signal will remain stored in the underlying buffer until the client is ready.
    ///
    /// Use `with_buffer` to populate and manipulate the buffer with the data you want.
    pub fn signal(&mut self, mut with_buffer: impl FnMut(&mut B)) {
        self.instances.retain(|_, instance| {
            with_buffer(&mut instance.buffer);
            if instance.ready {
                if let Some(data) = instance.buffer.next() {
                    instance.ready = false;
                    return instance.sender.send(Ok(data)).is_ok();
                }
            }

            true
        })
    }

    pub fn connect(&mut self, id: ClientSignalId, sender: UnboundedSender<Result<T, Status>>) {
        self.instances.insert(
            id,
            SignalInstance {
                sender,
                ready: true,
                buffer: B::default(),
            },
        );
    }

    fn disconnect(&mut self, id: ClientSignalId) {
        self.instances.remove(&id);
    }

    /// Mark this signal as ready to send.
    ///
    /// If there are signals already in the buffer, they will be sent.
    fn ready(&mut self, id: ClientSignalId) {
        let Some(instance) = self.instances.get_mut(&id) else {
            return;
        };

        if let Some(data) = instance.buffer.next() {
            instance.ready = false;
            if instance.sender.send(Ok(data)).is_err() {
                self.instances.remove(&id);
            }
        } else {
            instance.ready = true;
        }
    }
}

fn start_signal_stream<I, O, B, F>(
    sender: StateFnSender,
    in_stream: Streaming<I>,
    signal_data_selector: F,
) -> Result<Response<ResponseStream<O>>, Status>
where
    I: SignalRequest + std::fmt::Debug + Send + 'static,
    O: Send + 'static,
    B: SignalBuffer<O>,
    F: Fn(&mut State) -> &mut SignalData<O, B> + Clone + Send + 'static,
{
    let signal_data_selector_clone = signal_data_selector.clone();

    let client_signal_id = CLIENT_SIGNAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    run_bidirectional_streaming(
        sender,
        in_stream,
        move |state, request| {
            debug!("Got {request:?} from client stream");

            let signal = signal_data_selector(state);
            match request.control() {
                StreamControl::Ready => signal.ready(client_signal_id),
                StreamControl::Disconnect => signal.disconnect(client_signal_id),
                StreamControl::Unspecified => warn!("Received unspecified stream control"),
            }
        },
        move |state, sender, _join_handle| {
            let signal = signal_data_selector_clone(state);
            signal.connect(client_signal_id, sender);
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
    type OutputDisconnectStream = ResponseStream<OutputDisconnectResponse>;
    type OutputResizeStream = ResponseStream<OutputResizeResponse>;
    type OutputMoveStream = ResponseStream<OutputMoveResponse>;

    type WindowPointerEnterStream = ResponseStream<WindowPointerEnterResponse>;
    type WindowPointerLeaveStream = ResponseStream<WindowPointerLeaveResponse>;

    type TagActiveStream = ResponseStream<TagActiveResponse>;

    async fn output_connect(
        &self,
        request: Request<Streaming<OutputConnectRequest>>,
    ) -> Result<Response<Self::OutputConnectStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_connect
        })
    }

    async fn output_disconnect(
        &self,
        request: Request<Streaming<OutputDisconnectRequest>>,
    ) -> Result<Response<Self::OutputDisconnectStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_disconnect
        })
    }

    async fn output_resize(
        &self,
        request: Request<Streaming<OutputResizeRequest>>,
    ) -> Result<Response<Self::OutputResizeStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_resize
        })
    }

    async fn output_move(
        &self,
        request: Request<Streaming<OutputMoveRequest>>,
    ) -> Result<Response<Self::OutputMoveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_move
        })
    }

    async fn window_pointer_enter(
        &self,
        request: Request<Streaming<WindowPointerEnterRequest>>,
    ) -> Result<Response<Self::WindowPointerEnterStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_pointer_enter
        })
    }

    async fn window_pointer_leave(
        &self,
        request: Request<Streaming<WindowPointerLeaveRequest>>,
    ) -> Result<Response<Self::WindowPointerLeaveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_pointer_leave
        })
    }

    async fn tag_active(
        &self,
        request: Request<Streaming<TagActiveRequest>>,
    ) -> Result<Response<Self::TagActiveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.tag_active
        })
    }
}
