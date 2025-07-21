use smithay_client_toolkit::reexports::{
    client::{Dispatch, event_created_child},
    protocols::ext::foreign_toplevel_list::v1::client::{
        ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
    },
};

use crate::state::State;

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        _event: <ExtForeignToplevelListV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        // TODO:
    }

    event_created_child!(State, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ExtForeignToplevelHandleV1,
        _event: <ExtForeignToplevelHandleV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _data: &(),
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qhandle: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
    ) {
        // TODO:
    }
}
