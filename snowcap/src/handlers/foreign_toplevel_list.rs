use smithay_client_toolkit::reexports::{
    client::{Dispatch, event_created_child},
    protocols::ext::foreign_toplevel_list::v1::client::{
        ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
        ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
    },
};

use crate::state::State;

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                state.foreign_toplevel_list_handles.push((
                    toplevel,
                    ForeignToplevelListHandleData {
                        pending: None,
                        identifier: None,
                    },
                ));
            }
            ext_foreign_toplevel_list_v1::Event::Finished => (),
            _ => unreachable!(),
        }
    }

    event_created_child!(State, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state
                    .foreign_toplevel_list_handles
                    .retain(|(handle, _)| handle != proxy);
                state
                    .decorations
                    .retain(|deco| &deco.foreign_toplevel_list_handle != proxy);
                proxy.destroy();
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                let Some((_, ident)) = state
                    .foreign_toplevel_list_handles
                    .iter_mut()
                    .find(|(handle, _)| handle == proxy)
                else {
                    return;
                };

                ident.pending = Some(identifier);
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                let Some((_, ident)) = state
                    .foreign_toplevel_list_handles
                    .iter_mut()
                    .find(|(handle, _)| handle == proxy)
                else {
                    return;
                };

                ident.done();
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title: _ } => (),
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id: _ } => (),
            _ => unreachable!(),
        }
    }
}

pub struct ForeignToplevelListHandleData {
    pending: Option<String>,
    identifier: Option<String>,
}

impl ForeignToplevelListHandleData {
    fn done(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.identifier = Some(pending);
        }
    }

    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }
}
