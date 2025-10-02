use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicU32, Ordering},
};

use pinnacle_api_defs::pinnacle::signal::{
    self,
    v1::{
        InputDeviceAddedRequest, InputDeviceAddedResponse, OutputConnectRequest,
        OutputConnectResponse, OutputDisconnectRequest, OutputDisconnectResponse,
        OutputFocusedRequest, OutputFocusedResponse, OutputMoveRequest, OutputMoveResponse,
        OutputPointerEnterRequest, OutputPointerEnterResponse, OutputPointerLeaveRequest,
        OutputPointerLeaveResponse, OutputResizeRequest, OutputResizeResponse, SignalRequest,
        StreamControl, TagActiveRequest, TagActiveResponse, WindowFocusedRequest,
        WindowFocusedResponse, WindowPointerEnterRequest, WindowPointerEnterResponse,
        WindowPointerLeaveRequest, WindowPointerLeaveResponse, WindowSetFloatingRequest,
        WindowSetFloatingResponse, WindowSetFullscreenRequest, WindowSetFullscreenResponse,
        WindowSetMaximizedRequest, WindowSetMaximizedResponse, WindowSetSpilledRequest,
        WindowSetSpilledResponse, WindowSetTiledRequest, WindowSetTiledResponse,
        WindowTitleChangedRequest, WindowTitleChangedResponse, WindowUnsetFloatingRequest,
        WindowUnsetFloatingResponse, WindowUnsetFullscreenRequest, WindowUnsetFullscreenResponse,
        WindowUnsetMaximizedRequest, WindowUnsetMaximizedResponse, WindowUnsetSpilledRequest,
        WindowUnsetSpilledResponse, WindowUnsetTiledRequest, WindowUnsetTiledResponse,
    },
};
use smithay::output::Output;
use tonic::{Request, Response, Status, Streaming};
use tracing::warn;

use crate::{
    api::Sender,
    state::{State, WithState},
    tag::Tag,
    window::WindowElement,
};

use super::{ResponseStream, StateFnSender, run_bidirectional_streaming};

#[derive(Debug, Default)]
pub struct SignalState {
    // Output
    pub output_connect: OutputConnect,
    pub output_disconnect: OutputDisconnect,
    pub output_resize: OutputResize,
    pub output_move: OutputMove,
    pub output_pointer_enter: OutputPointerEnter,
    pub output_pointer_leave: OutputPointerLeave,
    pub output_focused: OutputFocused,

    // Window
    pub window_pointer_enter: WindowPointerEnter,
    pub window_pointer_leave: WindowPointerLeave,
    pub window_focused: WindowFocused,
    pub window_title_changed: WindowTitleChanged,
    pub window_set_floating: WindowSetFloating,
    pub window_unset_floating: WindowUnsetFloating,
    pub window_set_tiled: WindowSetTiled,
    pub window_unset_tiled: WindowUnsetTiled,
    pub window_set_maximized: WindowSetMaximized,
    pub window_unset_maximized: WindowUnsetMaximized,
    pub window_set_fullscreen: WindowSetFullscreen,
    pub window_unset_fullscreen: WindowUnsetFullscreen,
    pub window_set_spilled: WindowSetSpilled,
    pub window_unset_spilled: WindowUnsetSpilled,

    // Tag
    pub tag_active: TagActive,

    // Input
    pub input_device_added: InputDeviceAdded,
}

impl SignalState {
    pub fn clear(&mut self) {
        self.output_connect.clear();
        self.output_disconnect.clear();
        self.output_resize.clear();
        self.output_move.clear();
        self.output_pointer_enter.clear();
        self.output_pointer_leave.clear();
        self.output_focused.clear();

        self.window_pointer_enter.clear();
        self.window_pointer_leave.clear();
        self.window_focused.clear();
        self.window_title_changed.clear();
        self.window_set_floating.clear();
        self.window_unset_floating.clear();
        self.window_set_tiled.clear();
        self.window_unset_tiled.clear();
        self.window_set_maximized.clear();
        self.window_unset_maximized.clear();
        self.window_set_fullscreen.clear();
        self.window_unset_fullscreen.clear();
        self.window_set_spilled.clear();
        self.window_unset_spilled.clear();

        self.tag_active.clear();

        self.input_device_added.clear();
    }
}

#[derive(Debug, Default)]
pub struct SignalData<T> {
    instances: HashMap<ClientSignalId, SignalInstance<T>>,
}

#[derive(Debug)]
struct SignalInstance<T> {
    sender: Sender<Result<T, Status>>,
    ready: bool,
    buffer: VecDeque<T>,
}

pub trait Signal {
    type Args<'a>;

    fn signal(&mut self, args: Self::Args<'_>);
    fn clear(&mut self);
}

#[derive(Debug, Default)]
pub struct OutputConnect {
    v1: SignalData<OutputConnectResponse>,
}

impl Signal for OutputConnect {
    type Args<'a> = &'a smithay::output::Output;

    fn signal(&mut self, args: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(OutputConnectResponse {
                output_name: args.name(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputDisconnect {
    v1: SignalData<signal::v1::OutputDisconnectResponse>,
}

impl Signal for OutputDisconnect {
    type Args<'a> = &'a smithay::output::Output;

    fn signal(&mut self, args: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputDisconnectResponse {
                output_name: args.name(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputResize {
    v1: SignalData<signal::v1::OutputResizeResponse>,
}

impl Signal for OutputResize {
    type Args<'a> = (&'a smithay::output::Output, u32, u32);

    /// Args: (output, width, height)
    fn signal(&mut self, args: Self::Args<'_>) {
        let (output, w, h) = args;
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputResizeResponse {
                output_name: output.name(),
                logical_width: w,
                logical_height: h,
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputMove {
    v1: SignalData<signal::v1::OutputMoveResponse>,
}

impl Signal for OutputMove {
    type Args<'a> = &'a smithay::output::Output;

    fn signal(&mut self, output: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputMoveResponse {
                output_name: output.name(),
                x: output.current_location().x,
                y: output.current_location().y,
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputPointerEnter {
    v1: SignalData<signal::v1::OutputPointerEnterResponse>,
}

impl Signal for OutputPointerEnter {
    type Args<'a> = &'a Output;

    fn signal(&mut self, output: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputPointerEnterResponse {
                output_name: output.name(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputPointerLeave {
    v1: SignalData<signal::v1::OutputPointerLeaveResponse>,
}

impl Signal for OutputPointerLeave {
    type Args<'a> = &'a Output;

    fn signal(&mut self, output: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputPointerLeaveResponse {
                output_name: output.name(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct OutputFocused {
    v1: SignalData<signal::v1::OutputFocusedResponse>,
}

impl Signal for OutputFocused {
    type Args<'a> = &'a Output;

    fn signal(&mut self, output: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::OutputFocusedResponse {
                output_name: output.name(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowPointerEnter {
    v1: SignalData<signal::v1::WindowPointerEnterResponse>,
}

impl Signal for WindowPointerEnter {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowPointerEnterResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowPointerLeave {
    v1: SignalData<signal::v1::WindowPointerLeaveResponse>,
}

impl Signal for WindowPointerLeave {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowPointerLeaveResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowFocused {
    v1: SignalData<signal::v1::WindowFocusedResponse>,
}

impl Signal for WindowFocused {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowFocusedResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowTitleChanged {
    v1: SignalData<signal::v1::WindowTitleChangedResponse>,
}

impl Signal for WindowTitleChanged {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowTitleChangedResponse {
                window_id: window.with_state(|state| state.id.0),
                title: window.title().unwrap_or_default(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowSetFloating {
    v1: SignalData<signal::v1::WindowSetFloatingResponse>,
}

impl Signal for WindowSetFloating {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowSetFloatingResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowUnsetFloating {
    v1: SignalData<signal::v1::WindowUnsetFloatingResponse>,
}

impl Signal for WindowUnsetFloating {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowUnsetFloatingResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowSetTiled {
    v1: SignalData<signal::v1::WindowSetTiledResponse>,
}

impl Signal for WindowSetTiled {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowSetTiledResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowUnsetTiled {
    v1: SignalData<signal::v1::WindowUnsetTiledResponse>,
}

impl Signal for WindowUnsetTiled {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowUnsetTiledResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowSetMaximized {
    v1: SignalData<signal::v1::WindowSetMaximizedResponse>,
}

impl Signal for WindowSetMaximized {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowSetMaximizedResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowUnsetMaximized {
    v1: SignalData<signal::v1::WindowUnsetMaximizedResponse>,
}

impl Signal for WindowUnsetMaximized {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowUnsetMaximizedResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowSetFullscreen {
    v1: SignalData<signal::v1::WindowSetFullscreenResponse>,
}

impl Signal for WindowSetFullscreen {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowSetFullscreenResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowUnsetFullscreen {
    v1: SignalData<signal::v1::WindowUnsetFullscreenResponse>,
}

impl Signal for WindowUnsetFullscreen {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowUnsetFullscreenResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowSetSpilled {
    v1: SignalData<signal::v1::WindowSetSpilledResponse>,
}

impl Signal for WindowSetSpilled {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowSetSpilledResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct WindowUnsetSpilled {
    v1: SignalData<signal::v1::WindowUnsetSpilledResponse>,
}

impl Signal for WindowUnsetSpilled {
    type Args<'a> = &'a WindowElement;

    fn signal(&mut self, window: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::WindowUnsetSpilledResponse {
                window_id: window.with_state(|state| state.id.0),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct TagActive {
    v1: SignalData<signal::v1::TagActiveResponse>,
}

impl Signal for TagActive {
    type Args<'a> = &'a Tag;

    fn signal(&mut self, tag: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::TagActiveResponse {
                tag_id: tag.id().to_inner(),
                active: tag.active(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

#[derive(Debug, Default)]
pub struct InputDeviceAdded {
    v1: SignalData<signal::v1::InputDeviceAddedResponse>,
}

impl Signal for InputDeviceAdded {
    type Args<'a> = &'a smithay::reexports::input::Device;

    fn signal(&mut self, device: Self::Args<'_>) {
        self.v1.signal(|buf| {
            buf.push_back(signal::v1::InputDeviceAddedResponse {
                device_sysname: device.sysname().to_string(),
            });
        });
    }

    fn clear(&mut self) {
        self.v1.instances.clear();
    }
}

////////////////////////////////////////////////////

type ClientSignalId = u32;

static CLIENT_SIGNAL_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

impl<T> SignalData<T> {
    /// Attempt to send a signal.
    ///
    /// If the client is ready to accept more of this signal, it will be sent immediately.
    /// Otherwise, the signal will remain stored in the underlying buffer until the client is ready.
    ///
    /// Use `with_buffer` to populate and manipulate the buffer with the data you want.
    fn signal(&mut self, mut with_buffer: impl FnMut(&mut VecDeque<T>)) {
        self.instances.retain(|_, instance| {
            with_buffer(&mut instance.buffer);
            if instance.ready
                && let Some(data) = instance.buffer.pop_front()
            {
                instance.ready = false;
                return instance.sender.send_blocking(Ok(data)).is_ok();
            }

            true
        })
    }

    fn connect(&mut self, id: ClientSignalId, sender: Sender<Result<T, Status>>) {
        self.instances.insert(
            id,
            SignalInstance {
                sender,
                ready: true,
                buffer: Default::default(),
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

        if let Some(data) = instance.buffer.pop_front() {
            instance.ready = false;
            if instance.sender.send_blocking(Ok(data)).is_err() {
                self.instances.remove(&id);
            }
        } else {
            instance.ready = true;
        }
    }
}

fn start_signal_stream<I, O, F>(
    sender: StateFnSender,
    in_stream: Streaming<I>,
    signal_data_selector: F,
) -> Result<Response<ResponseStream<O>>, Status>
where
    I: SignalRequest + std::fmt::Debug + Send + 'static,
    O: Send + 'static,
    F: Fn(&mut State) -> &mut SignalData<O> + Clone + Send + 'static,
{
    let signal_data_selector_clone = signal_data_selector.clone();

    let client_signal_id = CLIENT_SIGNAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    run_bidirectional_streaming(
        sender,
        in_stream,
        move |state, request| {
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
impl signal::v1::signal_service_server::SignalService for SignalService {
    type OutputConnectStream = ResponseStream<OutputConnectResponse>;
    type OutputDisconnectStream = ResponseStream<OutputDisconnectResponse>;
    type OutputResizeStream = ResponseStream<OutputResizeResponse>;
    type OutputMoveStream = ResponseStream<OutputMoveResponse>;
    type OutputPointerEnterStream = ResponseStream<OutputPointerEnterResponse>;
    type OutputPointerLeaveStream = ResponseStream<OutputPointerLeaveResponse>;
    type OutputFocusedStream = ResponseStream<OutputFocusedResponse>;

    type WindowPointerEnterStream = ResponseStream<WindowPointerEnterResponse>;
    type WindowPointerLeaveStream = ResponseStream<WindowPointerLeaveResponse>;
    type WindowFocusedStream = ResponseStream<WindowFocusedResponse>;
    type WindowTitleChangedStream = ResponseStream<WindowTitleChangedResponse>;
    type WindowSetTiledStream = ResponseStream<WindowSetTiledResponse>;
    type WindowUnsetTiledStream = ResponseStream<WindowUnsetTiledResponse>;
    type WindowSetFloatingStream = ResponseStream<WindowSetFloatingResponse>;
    type WindowUnsetFloatingStream = ResponseStream<WindowUnsetFloatingResponse>;
    type WindowSetMaximizedStream = ResponseStream<WindowSetMaximizedResponse>;
    type WindowUnsetMaximizedStream = ResponseStream<WindowUnsetMaximizedResponse>;
    type WindowSetFullscreenStream = ResponseStream<WindowSetFullscreenResponse>;
    type WindowUnsetFullscreenStream = ResponseStream<WindowUnsetFullscreenResponse>;
    type WindowSetSpilledStream = ResponseStream<WindowSetSpilledResponse>;
    type WindowUnsetSpilledStream = ResponseStream<WindowUnsetSpilledResponse>;

    type TagActiveStream = ResponseStream<TagActiveResponse>;

    type InputDeviceAddedStream = ResponseStream<InputDeviceAddedResponse>;

    async fn output_connect(
        &self,
        request: Request<Streaming<OutputConnectRequest>>,
    ) -> Result<Response<Self::OutputConnectStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_connect.v1
        })
    }

    async fn output_disconnect(
        &self,
        request: Request<Streaming<OutputDisconnectRequest>>,
    ) -> Result<Response<Self::OutputDisconnectStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_disconnect.v1
        })
    }

    async fn output_resize(
        &self,
        request: Request<Streaming<OutputResizeRequest>>,
    ) -> Result<Response<Self::OutputResizeStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_resize.v1
        })
    }

    async fn output_move(
        &self,
        request: Request<Streaming<OutputMoveRequest>>,
    ) -> Result<Response<Self::OutputMoveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_move.v1
        })
    }

    async fn output_pointer_enter(
        &self,
        request: Request<Streaming<OutputPointerEnterRequest>>,
    ) -> Result<Response<Self::OutputPointerEnterStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_pointer_enter.v1
        })
    }

    async fn output_pointer_leave(
        &self,
        request: Request<Streaming<OutputPointerLeaveRequest>>,
    ) -> Result<Response<Self::OutputPointerLeaveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_pointer_leave.v1
        })
    }

    async fn output_focused(
        &self,
        request: Request<Streaming<OutputFocusedRequest>>,
    ) -> Result<Response<Self::OutputFocusedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.output_focused.v1
        })
    }

    async fn window_pointer_enter(
        &self,
        request: Request<Streaming<WindowPointerEnterRequest>>,
    ) -> Result<Response<Self::WindowPointerEnterStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_pointer_enter.v1
        })
    }

    async fn window_pointer_leave(
        &self,
        request: Request<Streaming<WindowPointerLeaveRequest>>,
    ) -> Result<Response<Self::WindowPointerLeaveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_pointer_leave.v1
        })
    }

    async fn window_focused(
        &self,
        request: Request<Streaming<WindowFocusedRequest>>,
    ) -> Result<Response<Self::WindowFocusedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_focused.v1
        })
    }

    async fn window_title_changed(
        &self,
        request: Request<Streaming<WindowTitleChangedRequest>>,
    ) -> Result<Response<Self::WindowTitleChangedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_title_changed.v1
        })
    }

    async fn window_set_tiled(
        &self,
        request: Request<Streaming<WindowSetTiledRequest>>,
    ) -> Result<Response<Self::WindowSetTiledStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_set_tiled.v1
        })
    }

    async fn window_unset_tiled(
        &self,
        request: Request<Streaming<WindowUnsetTiledRequest>>,
    ) -> Result<Response<Self::WindowUnsetTiledStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_unset_tiled.v1
        })
    }

    async fn window_set_floating(
        &self,
        request: Request<Streaming<WindowSetFloatingRequest>>,
    ) -> Result<Response<Self::WindowSetFloatingStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_set_floating.v1
        })
    }

    async fn window_unset_floating(
        &self,
        request: Request<Streaming<WindowUnsetFloatingRequest>>,
    ) -> Result<Response<Self::WindowUnsetFloatingStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_unset_floating.v1
        })
    }

    async fn window_set_maximized(
        &self,
        request: Request<Streaming<WindowSetMaximizedRequest>>,
    ) -> Result<Response<Self::WindowSetMaximizedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_set_maximized.v1
        })
    }

    async fn window_unset_maximized(
        &self,
        request: Request<Streaming<WindowUnsetMaximizedRequest>>,
    ) -> Result<Response<Self::WindowUnsetMaximizedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_unset_maximized.v1
        })
    }

    async fn window_set_fullscreen(
        &self,
        request: Request<Streaming<WindowSetFullscreenRequest>>,
    ) -> Result<Response<Self::WindowSetFullscreenStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_set_fullscreen.v1
        })
    }

    async fn window_unset_fullscreen(
        &self,
        request: Request<Streaming<WindowUnsetFullscreenRequest>>,
    ) -> Result<Response<Self::WindowUnsetFullscreenStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_unset_fullscreen.v1
        })
    }

    async fn window_set_spilled(
        &self,
        request: Request<Streaming<WindowSetSpilledRequest>>,
    ) -> Result<Response<Self::WindowSetSpilledStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_set_spilled.v1
        })
    }

    async fn window_unset_spilled(
        &self,
        request: Request<Streaming<WindowUnsetSpilledRequest>>,
    ) -> Result<Response<Self::WindowUnsetSpilledStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.window_unset_spilled.v1
        })
    }

    async fn tag_active(
        &self,
        request: Request<Streaming<TagActiveRequest>>,
    ) -> Result<Response<Self::TagActiveStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.tag_active.v1
        })
    }

    async fn input_device_added(
        &self,
        request: Request<Streaming<InputDeviceAddedRequest>>,
    ) -> Result<Response<Self::InputDeviceAddedStream>, Status> {
        let in_stream = request.into_inner();

        start_signal_stream(self.sender.clone(), in_stream, |state| {
            &mut state.pinnacle.signal_state.input_device_added.v1
        })
    }
}
