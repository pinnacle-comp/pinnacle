use smithay::{
    reexports::{
        calloop::Interest,
        wayland_server::{Resource, protocol::wl_surface::WlSurface},
    },
    utils::HookId,
    wayland::{
        compositor::{self, BufferAssignment, CompositorHandler, SurfaceAttributes},
        dmabuf,
        shell::xdg::{ToplevelSurface, XdgToplevelSurfaceData},
    },
};
use tracing::{error, field::Empty, trace, trace_span};

use crate::state::{Pinnacle, State, WithState};

#[cfg(feature = "snowcap")]
pub fn add_decoration_pre_commit_hook(deco: &crate::decoration::DecorationSurface) -> HookId {
    let deco = deco.clone();

    // FIXME: probably leaking the arc here
    compositor::add_pre_commit_hook::<State, _>(
        &deco.wl_surface().clone(),
        move |state, _dh, surface| {
            let _span = tracy_client::span!("mapped decoration pre-commit");
            let span =
                trace_span!("deco pre-commit", surface = %surface.id(), serial = Empty).entered();

            let (commit_serial, dmabuf) = compositor::with_states(surface, |states| {
                let dmabuf = {
                    let mut guard = states.cached_state.get::<SurfaceAttributes>();
                    match guard.pending().buffer.as_ref() {
                        Some(BufferAssignment::NewBuffer(buffer)) => {
                            let dmabuf = smithay::wayland::dmabuf::get_dmabuf(buffer).cloned().ok();
                            dmabuf
                        }
                        _ => None,
                    }
                };

                let role = states
                    .data_map
                    .get::<crate::protocol::snowcap_decoration::DecorationSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap();

                (role.configure_serial, dmabuf)
            });

            let mut transaction_for_dmabuf = None;
            if let Some(serial) = commit_serial {
                if !span.is_disabled() {
                    span.record("serial", format!("{serial:?}"));
                }

                if let Some(transaction) = deco.take_pending_transaction(serial) {
                    // Transaction can be already completed if it ran past the deadline.
                    if !transaction.is_completed() {
                        let is_last = transaction.is_last();

                        // If this is the last transaction, we don't need to add a separate
                        // notification, because the transaction will complete in our dmabuf blocker
                        // callback, which already calls blocker_cleared(), or by the end of this
                        // function, in which case there would be no blocker in the first place.
                        if !is_last {
                            // Waiting for some other surface; register a notification and add a
                            // transaction blocker.

                            if let Some(client) = surface.client() {
                                transaction.add_notification(
                                    state.pinnacle.blocker_cleared_tx.clone(),
                                    client.clone(),
                                );
                                compositor::add_blocker(surface, transaction.blocker());
                            }
                        }

                        transaction_for_dmabuf = Some(transaction);
                    }
                }
            }

            if let Some((blocker, source)) =
                dmabuf.and_then(|dmabuf| dmabuf.generate_blocker(Interest::READ).ok())
                && let Some(client) = surface.client()
            {
                let res = state
                    .pinnacle
                    .loop_handle
                    .insert_source(source, move |_, _, state| {
                        // This surface is now ready for the transaction.
                        drop(transaction_for_dmabuf.take());

                        let display_handle = state.pinnacle.display_handle.clone();
                        state
                            .client_compositor_state(&client)
                            .blocker_cleared(state, &display_handle);

                        Ok(())
                    });
                if res.is_ok() {
                    compositor::add_blocker(surface, blocker);
                    trace!("added dmabuf blocker");
                }
            }
        },
    )
}

/// Adds a pre-commit hook for mapped toplevels that blocks windows when transactions are pending.
///
/// It also takes over the role of the default dmabuf pre-commit hook, so when adding this
/// be sure to remove the default hook.
//
// Yoinked from niri
pub fn add_mapped_toplevel_pre_commit_hook(toplevel: &ToplevelSurface) -> HookId {
    compositor::add_pre_commit_hook::<State, _>(
        toplevel.wl_surface(),
        move |state, _dh, surface| {
            let _span = tracy_client::span!("mapped toplevel pre-commit");
            let span = trace_span!("toplevel pre-commit", surface = %surface.id(), serial = Empty)
                .entered();

            let Some(window) = state.pinnacle.window_for_surface(surface) else {
                error!("pre-commit hook for mapped surfaces must be removed upon unmapping");
                return;
            };

            let (got_unmapped, dmabuf, commit_serial) =
                compositor::with_states(surface, |states| {
                    let (got_unmapped, dmabuf) = {
                        let mut guard = states.cached_state.get::<SurfaceAttributes>();
                        match guard.pending().buffer.as_ref() {
                            Some(BufferAssignment::NewBuffer(buffer)) => {
                                let dmabuf =
                                    smithay::wayland::dmabuf::get_dmabuf(buffer).cloned().ok();
                                (false, dmabuf)
                            }
                            Some(BufferAssignment::Removed) => (true, None),
                            None => (false, None),
                        }
                    };

                    let role = states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap();

                    (got_unmapped, dmabuf, role.configure_serial)
                });

            #[cfg(feature = "snowcap")]
            let mut deco_serials = Vec::new();

            #[cfg(feature = "snowcap")]
            if let Some(geometry) = compositor::with_states(surface, |states| {
                let mut guard = states
                    .cached_state
                    .get::<smithay::wayland::shell::xdg::SurfaceCachedState>();
                guard.pending().geometry
            }) {
                let size = geometry.size;

                window.with_state(|state| {
                    for deco in state.decoration_surfaces.iter() {
                        deco.decoration_surface().with_pending_state(|state| {
                            state.toplevel_size = Some(size);
                        });
                        deco_serials.push(deco.decoration_surface().send_pending_configure());
                    }
                });
            }

            let mut transaction_for_dmabuf = None;
            if let Some(serial) = commit_serial {
                if !span.is_disabled() {
                    span.record("serial", format!("{serial:?}"));
                }

                #[cfg(feature = "snowcap")]
                let mut already_txned_deco = false;

                #[cfg(feature = "snowcap")]
                if window.with_state(|state| state.pending_transactions.is_empty()) {
                    use crate::util::transaction::TransactionBuilder;
                    use smithay::utils::Serial;

                    let txn_builder = TransactionBuilder::new();
                    let txn = txn_builder.get_transaction(&state.pinnacle.loop_handle);
                    window.with_state_mut(|state| {
                        for (deco, serial) in
                            state.decoration_surfaces.iter().zip(deco_serials.iter())
                        {
                            let Some(serial) = serial else {
                                continue;
                            };
                            deco.with_state_mut(|state| {
                                state.pending_transactions.push((*serial, txn.clone()))
                            });
                        }

                        state.pending_transactions.push((Serial::from(0), txn));

                        already_txned_deco = true;
                    });
                }

                trace!("taking pending transaction");
                if let Some(transaction) = window.take_pending_transaction(serial) {
                    #[cfg(feature = "snowcap")]
                    if !already_txned_deco {
                        window.with_state(|state| {
                            for (deco, serial) in state.decoration_surfaces.iter().zip(deco_serials)
                            {
                                let Some(serial) = serial else {
                                    continue;
                                };
                                deco.with_state_mut(|state| {
                                    state
                                        .pending_transactions
                                        .push((serial, transaction.clone()))
                                });
                            }
                        });
                    }

                    // Transaction can be already completed if it ran past the deadline.
                    if !transaction.is_completed() {
                        let is_last = transaction.is_last();

                        // If this is the last transaction, we don't need to add a separate
                        // notification, because the transaction will complete in our dmabuf blocker
                        // callback, which already calls blocker_cleared(), or by the end of this
                        // function, in which case there would be no blocker in the first place.
                        if !is_last {
                            // Waiting for some other surface; register a notification and add a
                            // transaction blocker.

                            if let Some(client) = surface.client() {
                                transaction.add_notification(
                                    state.pinnacle.blocker_cleared_tx.clone(),
                                    client.clone(),
                                );
                                compositor::add_blocker(surface, transaction.blocker());
                            }
                        }

                        // Delay dropping (and completing) the transaction until the dmabuf is ready.
                        // If there's no dmabuf, this will be dropped by the end of this pre-commit
                        // hook.
                        transaction_for_dmabuf = Some(transaction);
                    }
                }
            } else {
                error!("commit on a mapped surface without a configured serial");
            };

            if let Some((blocker, source)) =
                dmabuf.and_then(|dmabuf| dmabuf.generate_blocker(Interest::READ).ok())
                && let Some(client) = surface.client()
            {
                let res = state
                    .pinnacle
                    .loop_handle
                    .insert_source(source, move |_, _, state| {
                        // This surface is now ready for the transaction.
                        drop(transaction_for_dmabuf.take());

                        let display_handle = state.pinnacle.display_handle.clone();
                        state
                            .client_compositor_state(&client)
                            .blocker_cleared(state, &display_handle);

                        Ok(())
                    });
                if res.is_ok() {
                    compositor::add_blocker(surface, blocker);
                    trace!("added dmabuf blocker");
                }
            }

            if got_unmapped {
                let Some(output) = window.output(&state.pinnacle) else {
                    return;
                };

                state.backend.with_renderer(|renderer| {
                    window.capture_snapshot_and_store(
                        renderer,
                        output.current_scale().fractional_scale().into(),
                        1.0,
                    );
                });
            } else {
                window.with_state_mut(|state| state.snapshot.take());
            }
        },
    )
}

impl Pinnacle {
    /// Adds the default dmabuf pre-commit hook to a surface.
    ///
    /// If the surface belongs to a mapped window, this hook needs to be removed and
    /// the mapped hook added using [`add_mapped_toplevel_pre_commit_hook`].
    pub fn add_default_dmabuf_pre_commit_hook(&mut self, surface: &WlSurface) {
        let hook = compositor::add_pre_commit_hook::<State, _>(
            surface,
            |state, _display_handle, surface| {
                let maybe_dmabuf = compositor::with_states(surface, |surface_data| {
                    surface_data
                        .cached_state
                        .get::<SurfaceAttributes>()
                        .pending()
                        .buffer
                        .as_ref()
                        .and_then(|assignment| match assignment {
                            BufferAssignment::NewBuffer(buffer) => {
                                dmabuf::get_dmabuf(buffer).cloned().ok()
                            }
                            _ => None,
                        })
                });
                if let Some(dmabuf) = maybe_dmabuf
                    && let Ok((blocker, source)) = dmabuf.generate_blocker(Interest::READ)
                    && let Some(client) = surface.client()
                {
                    let res =
                        state
                            .pinnacle
                            .loop_handle
                            .insert_source(source, move |_, _, state| {
                                state
                                    .client_compositor_state(&client)
                                    .blocker_cleared(state, &state.pinnacle.display_handle.clone());
                                Ok(())
                            });
                    if res.is_ok() {
                        compositor::add_blocker(surface, blocker);
                    }
                }
            },
        );

        if let Some(prev_hook) = self.dmabuf_hooks.insert(surface.clone(), hook) {
            error!("tried to add dmabuf pre-commit hook when there already was one");
            compositor::remove_pre_commit_hook(surface, prev_hook);
        }
    }
}
