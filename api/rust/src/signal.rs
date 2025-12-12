//! Compositor signals.
//!
//! Your config can connect to various compositor signals that allow you to, for example, do
//! something when an output is connected or when the pointer enters a window.
//!
//! Some of the other modules have a `connect_signal` method that will allow you to pass in
//! callbacks to run on each signal. Use them to connect to the signals defined here.

#![allow(clippy::type_complexity)]

use std::{
    collections::{BTreeMap, btree_map},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use futures::{FutureExt, pin_mut};
use pinnacle_api_defs::pinnacle::signal::v1::{SignalRequest, StreamControl};
use tokio::sync::{
    mpsc::{UnboundedSender, unbounded_channel},
    oneshot,
};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};
use tonic::Streaming;

use crate::{
    BlockOnTokio,
    input::libinput::DeviceHandle,
    output::OutputHandle,
    tag::TagHandle,
    window::{LayoutMode, WindowHandle},
};

pub(crate) trait Signal {
    type Callback;
}

macro_rules! signals {
    ( $(
        $( #[$cfg_enum:meta] )* $enum:ident => {
            $(
                $( #[$cfg:meta] )* $name:ident = {
                    enum_name = $renamed:ident,
                    callback_type = $cb:ty,
                    client_request = $req:ident,
                    on_response = $on_resp:expr,
                }
            )*
        }
    )* ) => {$(
        $(
            $( #[$cfg] )*
            pub(crate) struct $name;

            impl $crate::signal::Signal for $name {
                type Callback = $cb;
            }

            impl SignalData<$name> {
                pub(crate) fn add_callback(&mut self, callback: <$name as Signal>::Callback) -> SignalHandle {
                    if self.callback_count.load(::std::sync::atomic::Ordering::SeqCst) == 0 {
                        self.connect()
                    }

                    let Some(callback_sender) = self.callback_sender.as_ref() else {
                        unreachable!("signal should already be connected here");
                    };

                    let Some(remove_callback_sender) = self.remove_callback_sender.clone() else {
                        unreachable!("signal should already be connected here");
                    };

                    callback_sender
                        .send((self.current_id, callback))
                        .expect("failed to send callback");

                    let handle = SignalHandle::new(self.current_id, remove_callback_sender);

                    self.current_id.0 += 1;

                    handle
                }

                fn reset(&mut self) {
                    self.callback_sender.take();
                    self.dc_pinger.take();
                    self.remove_callback_sender.take();
                    self.callback_count = Default::default();
                    self.current_id = SignalConnId::default();
                }

                fn connect(&mut self) {
                    self.reset();

                    let channels = connect_signal::<_, _, <$name as Signal>::Callback, _, _>(
                        self.callback_count.clone(),
                        |out| {
                            $crate::client::Client::signal().$req(out)
                                .block_on_tokio()
                                .expect("failed to request signal connection")
                                .into_inner()
                        },
                        $on_resp,
                    );

                    self.callback_sender.replace(channels.callback_sender);
                    self.dc_pinger.replace(channels.dc_pinger);
                    self.remove_callback_sender
                        .replace(channels.remove_callback_sender);
                }
            }
        )*

        $( #[$cfg_enum] )*
        pub enum $enum {
            $( $( #[$cfg] )* $renamed($cb),)*
        }
    )*};
}

signals! {
    /// Signals relating to output events.
    OutputSignal => {
        /// An output was connected.
        ///
        /// Callbacks receive the newly connected output.
        ///
        /// FIXME: This will not run on outputs that have been previously connected.
        /// |      Tell the dev to fix this in the compositor.
        OutputConnect = {
            enum_name = Connect,
            callback_type = SingleOutputFn,
            client_request = output_connect,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// An output was connected.
        ///
        /// Callbacks receive the disconnected output.
        OutputDisconnect = {
            enum_name = Disconnect,
            callback_type = SingleOutputFn,
            client_request = output_disconnect,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// An output's logical size changed.
        ///
        /// Callbacks receive the output and new width and height.
        OutputResize = {
            enum_name = Resize,
            callback_type = Box<dyn FnMut(&OutputHandle, u32, u32) + Send + 'static>,
            client_request = output_resize,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle, response.logical_width, response.logical_height)
                }
            },
        }
        /// An output's location in the global space changed.
        ///
        /// Callbacks receive the output and new x and y.
        OutputMove = {
            enum_name = Move,
            callback_type = Box<dyn FnMut(&OutputHandle, i32, i32) + Send + 'static>,
            client_request = output_move,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle, response.x, response.y)
                }
            },
        }
        /// The pointer entered an output.
        ///
        /// Callbacks receive the output the pointer entered.
        OutputPointerEnter = {
            enum_name = PointerEnter,
            callback_type = SingleOutputFn,
            client_request = output_pointer_enter,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// The pointer left an output.
        ///
        /// Callbacks receive the output the pointer left.
        OutputPointerLeave = {
            enum_name = PointerLeave,
            callback_type = SingleOutputFn,
            client_request = output_pointer_leave,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// The window got keyboard focus.
        ///
        /// Callbacks receive the newly focused window.
        OutputFocused = {
            enum_name = Focused,
            callback_type = SingleOutputFn,
            client_request = output_focused,
            on_response = |response, callbacks| {
                let handle = OutputHandle { name: response.output_name };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
    }
    /// Signals relating to window events.
    WindowSignal => {
        /// The pointer entered a window.
        ///
        /// Callbacks receive the window the pointer entered.
        WindowPointerEnter = {
            enum_name = PointerEnter,
            callback_type = SingleWindowFn,
            client_request = window_pointer_enter,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// The pointer left a window.
        ///
        /// Callbacks receive the window the pointer left.
        WindowPointerLeave = {
            enum_name = PointerLeave,
            callback_type = SingleWindowFn,
            client_request = window_pointer_leave,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// The window got keyboard focus.
        ///
        /// Callbacks receive the newly focused window.
        WindowFocused = {
            enum_name = Focused,
            callback_type = SingleWindowFn,
            client_request = window_focused,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// A window's title changed.
        ///
        /// Callbacks receive the window and new title.
        WindowTitleChanged = {
            enum_name = TitleChanged,
            callback_type = Box<dyn FnMut(&WindowHandle, &str) + Send + 'static>,
            client_request = window_title_changed,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };
                let title = response.title;

                for callback in callbacks {
                    callback(&handle, &title);
                }
            },
        }

        /// A window's layout mode changed.
        ///
        /// Callbacks receive the window and new layout mode.
        WindowLayoutModeChanged = {
            enum_name = LayoutModeChanged,
            callback_type = Box<dyn FnMut(&WindowHandle, LayoutMode) + Send + 'static>,
            client_request = window_layout_mode_changed,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };

                if let Ok(layout_mode) = response.layout_mode().try_into() {
                    for callback in callbacks {
                        callback(&handle, layout_mode);
                    }
                }
            },
        }


        /// A window was created (i.e., mapped for the first time).
        ///
        /// Callbacks receive the newly created window.
        WindowCreated = {
            enum_name = Created,
            callback_type = SingleWindowFn,
            client_request = window_created,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };
                for callback in callbacks {
                    callback(&handle);
                }
            },
        }

        /// A window was closed.
        ///
        /// Callbacks receive the window that was just closed, its title, and its app_id.
        /// Note: The window handle is no longer valid as the window was destroyed.
        /// Any subsequent operations on this handle will likely fail.
        WindowDestroyed = {
            enum_name = Destroyed,
            callback_type = Box<dyn FnMut(&WindowHandle, &str, &str) + Send + 'static>,
            client_request = window_destroyed,
            on_response = |response, callbacks| {
                let handle = WindowHandle { id: response.window_id };
                let title = response.title;
                let app_id = response.app_id;

                for callback in callbacks {
                    callback(&handle, &title, &app_id);
                }
            },
        }
    }
    /// Signals relating to tag events.
    TagSignal => {
        /// A tag was set to active or not active.
        TagActive = {
            enum_name = Active,
            callback_type = Box<dyn FnMut(&TagHandle, bool) + Send + 'static>,
            client_request = tag_active,
            on_response = |response, callbacks| {
                let handle = TagHandle { id: response.tag_id };

                for callback in callbacks {
                    callback(&handle, response.active);
                }
            },
        }
        /// A tag was created.
        TagCreated = {
            enum_name = Created,
            callback_type = Box<dyn FnMut(&TagHandle) + Send + 'static>,
            client_request = tag_created,
            on_response = |response, callbacks| {
                let handle = TagHandle { id: response.tag_id };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
        /// A tag was removed
        TagRemoved = {
            enum_name = Removed,
            callback_type = Box<dyn FnMut(&TagHandle) + Send + 'static>,
            client_request = tag_removed,
            on_response = |response, callbacks| {
                let handle = TagHandle { id: response.tag_id };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
    }
    /// Signals relating to input events.
    InputSignal => {
        /// A new input device was connected.
        InputDeviceAdded = {
            enum_name = DeviceAdded,
            callback_type = Box<dyn FnMut(&DeviceHandle) + Send + 'static>,
            client_request = input_device_added,
            on_response = |response, callbacks| {
                let handle = DeviceHandle { sysname: response.device_sysname };

                for callback in callbacks {
                    callback(&handle);
                }
            },
        }
    }
}

pub(crate) type SingleOutputFn = Box<dyn FnMut(&OutputHandle) + Send + 'static>;
pub(crate) type SingleWindowFn = Box<dyn FnMut(&WindowHandle) + Send + 'static>;

pub(crate) struct SignalState {
    pub(crate) output_connect: SignalData<OutputConnect>,
    pub(crate) output_disconnect: SignalData<OutputDisconnect>,
    pub(crate) output_resize: SignalData<OutputResize>,
    pub(crate) output_move: SignalData<OutputMove>,
    pub(crate) output_pointer_enter: SignalData<OutputPointerEnter>,
    pub(crate) output_pointer_leave: SignalData<OutputPointerLeave>,
    pub(crate) output_focused: SignalData<OutputFocused>,

    pub(crate) window_pointer_enter: SignalData<WindowPointerEnter>,
    pub(crate) window_pointer_leave: SignalData<WindowPointerLeave>,
    pub(crate) window_focused: SignalData<WindowFocused>,
    pub(crate) window_title_changed: SignalData<WindowTitleChanged>,
    pub(crate) window_layout_mode_changed: SignalData<WindowLayoutModeChanged>,
    pub(crate) window_created: SignalData<WindowCreated>,
    pub(crate) window_destroyed: SignalData<WindowDestroyed>,

    pub(crate) tag_active: SignalData<TagActive>,
    pub(crate) tag_created: SignalData<TagCreated>,
    pub(crate) tag_removed: SignalData<TagRemoved>,

    pub(crate) input_device_added: SignalData<InputDeviceAdded>,
}

impl std::fmt::Debug for SignalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalState").finish()
    }
}

impl SignalState {
    pub(crate) fn new() -> Self {
        Self {
            output_connect: SignalData::new(),
            output_disconnect: SignalData::new(),
            output_resize: SignalData::new(),
            output_move: SignalData::new(),
            output_pointer_enter: SignalData::new(),
            output_pointer_leave: SignalData::new(),
            output_focused: SignalData::new(),

            window_pointer_enter: SignalData::new(),
            window_pointer_leave: SignalData::new(),
            window_focused: SignalData::new(),
            window_title_changed: SignalData::new(),
            window_layout_mode_changed: SignalData::new(),
            window_created: SignalData::new(),
            window_destroyed: SignalData::new(),

            tag_active: SignalData::new(),
            tag_created: SignalData::new(),
            tag_removed: SignalData::new(),

            input_device_added: SignalData::new(),
        }
    }

    pub(crate) fn shutdown(&mut self) {
        self.output_connect.reset();
        self.output_disconnect.reset();
        self.output_resize.reset();
        self.output_move.reset();
        self.output_pointer_enter.reset();
        self.output_pointer_leave.reset();
        self.output_focused.reset();

        self.window_pointer_enter.reset();
        self.window_pointer_leave.reset();
        self.window_focused.reset();
        self.window_title_changed.reset();
        self.window_layout_mode_changed.reset();
        self.window_created.reset();
        self.window_destroyed.reset();

        self.tag_active.reset();
        self.tag_created.reset();
        self.tag_removed.reset();

        self.input_device_added.reset();
    }
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SignalConnId(pub(crate) u32);

pub(crate) struct SignalData<S: Signal> {
    callback_sender: Option<UnboundedSender<(SignalConnId, S::Callback)>>,
    remove_callback_sender: Option<UnboundedSender<SignalConnId>>,
    dc_pinger: Option<oneshot::Sender<()>>,
    current_id: SignalConnId,
    callback_count: Arc<AtomicU32>,
}

impl<S: Signal> SignalData<S> {
    fn new() -> Self {
        Self {
            callback_sender: Default::default(),
            remove_callback_sender: Default::default(),
            dc_pinger: Default::default(),
            current_id: Default::default(),
            callback_count: Default::default(),
        }
    }
}

struct ConnectSignalChannels<F> {
    callback_sender: UnboundedSender<(SignalConnId, F)>,
    dc_pinger: oneshot::Sender<()>,
    remove_callback_sender: UnboundedSender<SignalConnId>,
}

fn connect_signal<Req, Resp, F, T, O>(
    callback_count: Arc<AtomicU32>,
    to_in_stream: T,
    mut on_response: O,
) -> ConnectSignalChannels<F>
where
    Req: SignalRequest + Send + 'static,
    Resp: Send + 'static,
    F: Send + 'static,
    T: FnOnce(UnboundedReceiverStream<Req>) -> Streaming<Resp>,
    O: FnMut(Resp, btree_map::ValuesMut<'_, SignalConnId, F>) + Send + 'static,
{
    let (control_sender, recv) = unbounded_channel::<Req>();
    let out_stream = UnboundedReceiverStream::new(recv);

    let mut in_stream = to_in_stream(out_stream);

    let (callback_sender, mut callback_recv) = unbounded_channel::<(SignalConnId, F)>();
    let (remove_callback_sender, mut remove_callback_recv) = unbounded_channel::<SignalConnId>();
    let (dc_pinger, mut dc_ping_recv) = oneshot::channel::<()>();

    let signal_future = async move {
        let mut callbacks = BTreeMap::<SignalConnId, F>::new();

        control_sender
            .send(Req::from_control(StreamControl::Ready))
            .map_err(|err| {
                println!("{err}");
                err
            })
            .expect("send failed");

        loop {
            let in_stream_next = in_stream.next().fuse();
            pin_mut!(in_stream_next);
            let callback_recv_recv = callback_recv.recv().fuse();
            pin_mut!(callback_recv_recv);
            let remove_callback_recv_recv = remove_callback_recv.recv().fuse();
            pin_mut!(remove_callback_recv_recv);
            let mut dc_ping_recv_fuse = (&mut dc_ping_recv).fuse();

            futures::select! {
                response = in_stream_next => {
                    let Some(response) = response else { continue };

                    match response {
                        Ok(response) => {
                            on_response(response, callbacks.values_mut());

                            control_sender
                                .send(Req::from_control(StreamControl::Ready))
                                .expect("send failed");

                            tokio::task::yield_now().await;
                        }
                        Err(status) => eprintln!("Error in recv: {status}"),
                    }
                }
                callback = callback_recv_recv => {
                    if let Some((id, callback)) = callback {
                        callbacks.insert(id, callback);
                        callback_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
                remove = remove_callback_recv_recv => {
                    if let Some(id) = remove {
                        if callbacks.remove(&id).is_some() {
                            assert!(callback_count.fetch_sub(1, Ordering::SeqCst) > 0);
                        }
                        if callbacks.is_empty() {
                            assert!(callback_count.load(Ordering::SeqCst) == 0);
                            control_sender.send(Req::from_control(StreamControl::Disconnect)).expect("send failed");
                            break;
                        }
                    }
                }
                _dc = dc_ping_recv_fuse => {
                    let _ = control_sender.send(Req::from_control(StreamControl::Disconnect));
                    break;
                }
            }
        }
    };

    tokio::spawn(signal_future);

    ConnectSignalChannels {
        callback_sender,
        dc_pinger,
        remove_callback_sender,
    }
}

/// A handle that can be used to disconnect from a signal connection.
///
/// This will remove the connected callback.
#[derive(Debug, Clone)]
pub struct SignalHandle {
    id: SignalConnId,
    remove_callback_sender: UnboundedSender<SignalConnId>,
}

impl SignalHandle {
    pub(crate) fn new(
        id: SignalConnId,
        remove_callback_sender: UnboundedSender<SignalConnId>,
    ) -> Self {
        Self {
            id,
            remove_callback_sender,
        }
    }

    /// Disconnects this signal connection.
    pub fn disconnect(&self) {
        self.remove_callback_sender
            .send(self.id)
            .expect("failed to disconnect signal");
    }
}
