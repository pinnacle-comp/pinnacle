use std::sync::{Arc, Mutex, Weak};

use bitflags::bitflags;
use smithay_client_toolkit::{
    reexports::{
        client::{Dispatch, Proxy, event_created_child, protocol::wl_output::WlOutput},
        protocols_wlr::foreign_toplevel::v1::client::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
    },
    registry::GlobalProxy,
};

use crate::state::State;

pub struct Inner {
    _manager: GlobalProxy<ZwlrForeignToplevelManagerV1>,
    toplevels: Vec<ZwlrForeignToplevelHandleV1>,
}

#[derive(Clone)]
pub struct ZwlrForeignToplevelManagementState(Arc<Mutex<Inner>>);

#[derive(Clone)]
pub struct WeakZwlrForeignToplevelManagementState(Weak<Mutex<Inner>>);

#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ForeignToplevelInfo {
    pub app_id: String,
    pub title: String,
    pub outputs: Vec<WlOutput>, // TODO: Replace by the output id or name ?
    pub state: ToplevelState,
    // TODO: What about parents ?
}

#[derive(Debug, Default)]
pub struct ForeignToplevelInner {
    current_info: Option<ForeignToplevelInfo>,
    pending_info: ForeignToplevelInfo,
}

#[derive(Debug, Default, Clone)]
pub struct ForeignToplevelData(Arc<Mutex<ForeignToplevelInner>>);

#[derive(Debug, Clone)]
pub enum ZwlrForeignToplevelEvent {
    Added(ZwlrForeignToplevelHandleV1),
    Closed(ZwlrForeignToplevelHandleV1),
    Changed(ZwlrForeignToplevelHandleV1),
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ToplevelState: u32 {
        const None = 0;
        const Maximized = 1;
        const Minimized = 2;
        const Activated = 4;
        const Fullscreen = 8;
    }
}

impl ZwlrForeignToplevelManagementState {
    pub fn new(
        globals: &smithay_client_toolkit::reexports::client::globals::GlobalList,
        qh: &smithay_client_toolkit::reexports::client::QueueHandle<State>,
    ) -> Self {
        let _manager = GlobalProxy::from(globals.bind(qh, 1..=3, ()));

        Self(Arc::new(Mutex::new(Inner {
            _manager,
            toplevels: Vec::new(),
        })))
    }

    pub fn with_toplevels<F, Ret>(&self, processor: F) -> Ret
    where
        F: FnOnce(&[ZwlrForeignToplevelHandleV1]) -> Ret,
    {
        processor(&self.0.lock().unwrap().toplevels)
    }

    pub fn with_toplevels_mut<F, Ret>(&self, processor: F) -> Ret
    where
        F: Fn(&mut Vec<ZwlrForeignToplevelHandleV1>) -> Ret,
    {
        processor(&mut self.0.lock().unwrap().toplevels)
    }

    pub fn info(&self, toplevel: &ZwlrForeignToplevelHandleV1) -> Option<ForeignToplevelInfo> {
        toplevel
            .data::<ForeignToplevelData>()?
            .0
            .lock()
            .unwrap()
            .current_info
            .clone()
    }

    pub fn downgrade(&self) -> WeakZwlrForeignToplevelManagementState {
        WeakZwlrForeignToplevelManagementState(Arc::downgrade(&self.0))
    }
}

impl WeakZwlrForeignToplevelManagementState {
    pub fn upgrade(&self) -> Option<ZwlrForeignToplevelManagementState> {
        self.0.upgrade().map(ZwlrForeignToplevelManagementState)
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        _event: <ZwlrForeignToplevelManagerV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        // TODO:
    }

    event_created_child!(State, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, Default::default())
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ForeignToplevelData> for State {
    fn event(
        state: &mut Self,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: <ZwlrForeignToplevelHandleV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        data: &ForeignToplevelData,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.zwlr_toplevel_closed(proxy.clone());
                state
                    .zwlr_foreign_toplevel_mgmt_state
                    .with_toplevels_mut(|toplevels| {
                        toplevels.retain(|t| t != proxy);
                    });
                proxy.destroy();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.0.lock().unwrap().pending_info.title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.0.lock().unwrap().pending_info.app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                data.0.lock().unwrap().pending_info.outputs.push(output);
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                data.0
                    .lock()
                    .unwrap()
                    .pending_info
                    .outputs
                    .retain(|o| o != &output);
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: flags } => {
                data.0.lock().unwrap().pending_info.state = flags.into();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Parent { parent: _ } => (),
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                let mut inner = data.0.lock().unwrap();
                let new_toplevel = inner.current_info.is_none();
                inner.current_info = Some(inner.pending_info.clone());
                std::mem::drop(inner);

                if new_toplevel {
                    state
                        .zwlr_foreign_toplevel_mgmt_state
                        .with_toplevels_mut(|toplevels| {
                            toplevels.push(proxy.clone());
                        });
                    state.new_zwlr_toplevel(proxy.clone());
                } else {
                    state.zwlr_toplevel_updated(proxy.clone());
                }
            }
            _ => (),
        }
    }
}

impl State {
    pub fn new_zwlr_toplevel(&mut self, _handle: ZwlrForeignToplevelHandleV1) {}

    pub fn zwlr_toplevel_updated(&mut self, _handle: ZwlrForeignToplevelHandleV1) {}

    pub fn zwlr_toplevel_closed(&mut self, _handle: ZwlrForeignToplevelHandleV1) {}
}

impl ForeignToplevelData {
    pub fn with_info<F, Ret>(&self, processor: F) -> Option<Ret>
    where
        F: FnOnce(&ForeignToplevelInfo) -> Ret,
    {
        self.0.lock().ok()?.current_info.as_ref().map(processor)
    }
}

impl From<Vec<u8>> for ToplevelState {
    fn from(value: Vec<u8>) -> Self {
        value.iter().fold(Self::None, |acc, val| {
            let flag = match &val {
                0 => Self::Maximized,
                1 => Self::Minimized,
                2 => Self::Activated,
                3 => Self::Fullscreen,
                _ => Self::None,
            };

            acc | flag
        })
    }
}

impl Default for ToplevelState {
    fn default() -> Self {
        Self::None
    }
}
