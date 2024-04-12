//! Compositor signals.
//!
//! Your config can connect to various compositor signals that allow you to, for example, do
//! something when an output is connected or when the pointer enters a window.
//!
//! Some of the other modules have a `connect_signal` method that will allow you to pass in
//! callbacks to run on each signal. Use them to connect to the signals defined here.

#![allow(clippy::type_complexity)]

use std::{
    collections::{btree_map, BTreeMap},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use futures::{future::BoxFuture, pin_mut, FutureExt};
use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    signal_service_client::SignalServiceClient, SignalRequest, StreamControl,
};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedSender},
    oneshot,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tonic::{transport::Channel, Streaming};

use crate::{block_on_tokio, output::OutputHandle, window::WindowHandle, OUTPUT, WINDOW};

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
                    self.callback_count.store(0, Ordering::SeqCst);
                    self.current_id = SignalConnId::default();
                }

                fn connect(&mut self) {
                    self.reset();

                    let channels = connect_signal::<_, _, <$name as Signal>::Callback, _, _>(
                        &self.fut_sender,
                        self.callback_count.clone(),
                        |out| {
                            block_on_tokio(self.client.$req(out))
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
                if let Some(output_name) = response.output_name {
                    let output = OUTPUT.get().expect("OUTPUT doesn't exist");
                    let handle = output.new_handle(output_name);

                    for callback in callbacks {
                        callback(&handle);
                    }
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
                if let Some(output_name) = &response.output_name {
                    let output = OUTPUT.get().expect("OUTPUT doesn't exist");
                    let handle = output.new_handle(output_name);

                    for callback in callbacks {
                        callback(&handle, response.logical_width(), response.logical_height())
                    }
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
                if let Some(output_name) = &response.output_name {
                    let output = OUTPUT.get().expect("OUTPUT doesn't exist");
                    let handle = output.new_handle(output_name);

                    for callback in callbacks {
                        callback(&handle, response.x(), response.y())
                    }
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
                if let Some(window_id) = response.window_id {
                    let window = WINDOW.get().expect("WINDOW doesn't exist");
                    let handle = window.new_handle(window_id);

                    for callback in callbacks {
                        callback(&handle);
                    }
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
                if let Some(window_id) = response.window_id {
                    let window = WINDOW.get().expect("WINDOW doesn't exist");
                    let handle = window.new_handle(window_id);

                    for callback in callbacks {
                        callback(&handle);
                    }
                }
            },
        }
    }
}

pub(crate) type SingleOutputFn = Box<dyn FnMut(&OutputHandle) + Send + 'static>;
pub(crate) type SingleWindowFn = Box<dyn FnMut(&WindowHandle) + Send + 'static>;

pub(crate) struct SignalState {
    pub(crate) output_connect: SignalData<OutputConnect>,
    pub(crate) output_resize: SignalData<OutputResize>,
    pub(crate) output_move: SignalData<OutputMove>,
    pub(crate) window_pointer_enter: SignalData<WindowPointerEnter>,
    pub(crate) window_pointer_leave: SignalData<WindowPointerLeave>,
}

impl SignalState {
    pub(crate) fn new(
        channel: Channel,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Self {
        let client = SignalServiceClient::new(channel);
        Self {
            output_connect: SignalData::new(client.clone(), fut_sender.clone()),
            output_resize: SignalData::new(client.clone(), fut_sender.clone()),
            output_move: SignalData::new(client.clone(), fut_sender.clone()),
            window_pointer_enter: SignalData::new(client.clone(), fut_sender.clone()),
            window_pointer_leave: SignalData::new(client.clone(), fut_sender.clone()),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SignalConnId(pub(crate) u32);

pub(crate) struct SignalData<S: Signal> {
    client: SignalServiceClient<Channel>,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    callback_sender: Option<UnboundedSender<(SignalConnId, S::Callback)>>,
    remove_callback_sender: Option<UnboundedSender<SignalConnId>>,
    dc_pinger: Option<oneshot::Sender<()>>,
    current_id: SignalConnId,
    callback_count: Arc<AtomicU32>,
}

impl<S: Signal> SignalData<S> {
    fn new(
        client: SignalServiceClient<Channel>,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Self {
        Self {
            client,
            fut_sender,
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
    fut_sender: &UnboundedSender<BoxFuture<'static, ()>>,
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
                    control_sender.send(Req::from_control(StreamControl::Disconnect)).expect("send failed");
                    break;
                }
            }
        }
    };

    fut_sender.send(signal_future.boxed()).expect("send failed");

    ConnectSignalChannels {
        callback_sender,
        dc_pinger,
        remove_callback_sender,
    }
}

/// A handle that can be used to disconnect from a signal connection.
///
/// This will remove the connected callback.
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

    /// Disconnect this signal connection.
    pub fn disconnect(self) {
        self.remove_callback_sender
            .send(self.id)
            .expect("failed to disconnect signal");
    }
}
