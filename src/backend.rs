// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        renderer::{
            element::{
                default_primary_scanout_output_compare, utils::select_dmabuf_feedback,
                RenderElementStates,
            },
            gles::GlesRenderer,
            ImportDma, Renderer, TextureFilter,
        },
    },
    delegate_dmabuf,
    desktop::{
        layer_map_for_output,
        utils::{
            send_frames_surface_tree, surface_primary_scanout_output,
            update_surface_primary_scanout_output,
        },
        Space,
    },
    input::pointer::CursorImageStatus,
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        fractional_scale::with_fractional_scale,
    },
};
use tracing::error;

use crate::{
    state::{Pinnacle, State, SurfaceDmabufFeedback, WithState},
    window::WindowElement,
};

#[cfg(feature = "testing")]
use self::dummy::Dummy;
use self::{udev::Udev, winit::Winit};

#[cfg(feature = "testing")]
pub mod dummy;
pub mod udev;
pub mod winit;

pub enum Backend {
    /// The compositor is running in a Winit window
    Winit(Winit),
    /// The compositor is running in a tty
    Udev(Udev),
    #[cfg(feature = "testing")]
    Dummy(Dummy),
}

pub(crate) struct UninitBackend<B> {
    pub(crate) seat_name: String,
    #[allow(clippy::complexity)]
    pub(crate) init: Box<dyn FnOnce(&mut Pinnacle) -> anyhow::Result<B>>,
}

impl Backend {
    pub fn set_upscale_filter(&mut self, filter: TextureFilter) {
        match self {
            Backend::Winit(winit) => {
                if let Err(err) = winit.backend.renderer().upscale_filter(filter) {
                    error!("Failed to set winit upscale filter: {err}");
                }
            }
            Backend::Udev(udev) => udev.upscale_filter = filter,
            #[cfg(feature = "testing")]
            Backend::Dummy(_) => (),
        }
    }

    pub fn set_downscale_filter(&mut self, filter: TextureFilter) {
        match self {
            Backend::Winit(winit) => {
                if let Err(err) = winit.backend.renderer().downscale_filter(filter) {
                    error!("Failed to set winit upscale filter: {err}");
                }
            }
            Backend::Udev(udev) => udev.downscale_filter = filter,
            #[cfg(feature = "testing")]
            Backend::Dummy(_) => (),
        }
    }

    pub fn seat_name(&self) -> String {
        match self {
            Backend::Winit(winit) => winit.seat_name(),
            Backend::Udev(udev) => udev.seat_name(),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.seat_name(),
        }
    }

    pub fn early_import(&mut self, surface: &WlSurface) {
        match self {
            Backend::Winit(winit) => winit.early_import(surface),
            Backend::Udev(udev) => udev.early_import(surface),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.early_import(surface),
        }
    }

    pub fn with_renderer<T>(
        &mut self,
        with_renderer: impl FnOnce(&mut GlesRenderer) -> T,
    ) -> Option<T> {
        match self {
            Backend::Winit(winit) => Some(with_renderer(winit.backend.renderer())),
            Backend::Udev(udev) => Some(with_renderer(udev.renderer().ok()?.as_mut())),
            #[cfg(feature = "testing")]
            Backend::Dummy(_) => None,
        }
    }

    /// Returns `true` if the backend is [`Winit`].
    ///
    /// [`Winit`]: Backend::Winit
    #[must_use]
    pub fn is_winit(&self) -> bool {
        matches!(self, Self::Winit(..))
    }

    /// Returns `true` if the backend is [`Udev`].
    ///
    /// [`Udev`]: Backend::Udev
    #[must_use]
    pub fn is_udev(&self) -> bool {
        matches!(self, Self::Udev(..))
    }
}

pub trait BackendData: 'static {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);

    // INFO: only for udev in anvil, maybe shouldn't be a trait fn?
    fn early_import(&mut self, surface: &WlSurface);
}

impl BackendData for Backend {
    fn seat_name(&self) -> String {
        match self {
            Backend::Winit(winit) => winit.seat_name(),
            Backend::Udev(udev) => udev.seat_name(),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.seat_name(),
        }
    }

    fn reset_buffers(&mut self, output: &Output) {
        match self {
            Backend::Winit(winit) => winit.reset_buffers(output),
            Backend::Udev(udev) => udev.reset_buffers(output),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.reset_buffers(output),
        }
    }

    fn early_import(&mut self, surface: &WlSurface) {
        match self {
            Backend::Winit(winit) => winit.early_import(surface),
            Backend::Udev(udev) => udev.early_import(surface),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.early_import(surface),
        }
    }
}

/// Update surface primary scanout outputs and send frames and dmabuf feedback to visible windows
/// and layers.
pub fn post_repaint(
    output: &Output,
    render_element_states: &RenderElementStates,
    space: &Space<WindowElement>,
    dmabuf_feedback: Option<SurfaceDmabufFeedback<'_>>,
    time: Duration,
    cursor_status: &CursorImageStatus,
) {
    // let throttle = Some(Duration::from_secs(1));
    let throttle = Some(Duration::ZERO);

    space.elements().for_each(|window| {
        window.with_surfaces(|surface, states_inner| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states_inner,
                render_element_states,
                default_primary_scanout_output_compare,
            );

            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states_inner, |fraction_scale| {
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        if space.outputs_for_element(window).contains(output) {
            window.send_frame(output, time, throttle, surface_primary_scanout_output);
            if let Some(dmabuf_feedback) = dmabuf_feedback {
                window.send_dmabuf_feedback(
                    output,
                    surface_primary_scanout_output,
                    |surface, _| {
                        select_dmabuf_feedback(
                            surface,
                            render_element_states,
                            dmabuf_feedback.render_feedback,
                            dmabuf_feedback.scanout_feedback,
                        )
                    },
                );
            }
        }
    });

    let map = layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.with_surfaces(|surface, states| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states,
                render_element_states,
                default_primary_scanout_output_compare,
            );

            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fraction_scale| {
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        layer_surface.send_frame(output, time, throttle, surface_primary_scanout_output);
        if let Some(dmabuf_feedback) = dmabuf_feedback {
            layer_surface.send_dmabuf_feedback(
                output,
                surface_primary_scanout_output,
                |surface, _| {
                    select_dmabuf_feedback(
                        surface,
                        render_element_states,
                        dmabuf_feedback.render_feedback,
                        dmabuf_feedback.scanout_feedback,
                    )
                },
            );
        }
    }

    // Send frames to the cursor surface so it updates correctly
    if let CursorImageStatus::Surface(surf) = cursor_status {
        send_frames_surface_tree(surf, output, time, Some(Duration::ZERO), |_, _| None);
    }

    if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
        send_frames_surface_tree(
            lock_surface.wl_surface(),
            output,
            time,
            Some(Duration::ZERO),
            |_, _| None,
        );
    }
}

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        match &mut self.backend {
            Backend::Winit(winit) => &mut winit.dmabuf_state.0,
            Backend::Udev(udev) => {
                &mut udev
                    .dmabuf_state
                    .as_mut()
                    .expect("udev had no dmabuf state")
                    .0
            }
            #[cfg(feature = "testing")]
            Backend::Dummy(_) => unreachable!(),
        }
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        let res = match &mut self.backend {
            Backend::Winit(winit) => winit
                .backend
                .renderer()
                .import_dmabuf(&dmabuf, None)
                .map(|_| ())
                .map_err(|_| ()),
            Backend::Udev(udev) => udev
                .gpu_manager
                .single_renderer(&udev.primary_gpu)
                .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
                .map(|_| ())
                .map_err(|_| ()),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy
                .renderer
                .import_dmabuf(&dmabuf, None)
                .map(|_| ())
                .map_err(|_| ()),
        };

        if res.is_ok() {
            let _ = notifier.successful::<State>();
        } else {
            notifier.failed();
        }
    }
}
delegate_dmabuf!(State);
