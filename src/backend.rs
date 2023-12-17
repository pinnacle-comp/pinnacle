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
            ImportDma,
        },
    },
    delegate_dmabuf,
    desktop::{
        layer_map_for_output,
        utils::{surface_primary_scanout_output, update_surface_primary_scanout_output},
        Space,
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
        fractional_scale::with_fractional_scale,
    },
};

use crate::{
    state::{State, SurfaceDmabufFeedback},
    window::WindowElement,
};

use self::{udev::Udev, winit::Winit};

pub mod udev;
pub mod winit;

pub enum Backend {
    /// The compositor is running in a Winit window
    Winit(Winit),
    /// The compositor is running in a tty
    Udev(Udev),
}

impl Backend {
    pub fn seat_name(&self) -> String {
        match self {
            Backend::Winit(winit) => winit.seat_name(),
            Backend::Udev(udev) => udev.seat_name(),
        }
    }

    pub fn early_import(&mut self, surface: &WlSurface) {
        match self {
            Backend::Winit(winit) => winit.early_import(surface),
            Backend::Udev(udev) => udev.early_import(surface),
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

/// A trait defining common methods for each available backend: winit and tty-udev
pub trait BackendData: 'static {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);

    // INFO: only for udev in anvil, maybe shouldn't be a trait fn?
    fn early_import(&mut self, surface: &WlSurface);
}

/// Update surface primary scanout outputs and send frames and dmabuf feedback to visible windows
/// and layers.
pub fn post_repaint(
    output: &Output,
    render_element_states: &RenderElementStates,
    space: &Space<WindowElement>,
    dmabuf_feedback: Option<SurfaceDmabufFeedback<'_>>,
    time: Duration,
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
        }
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        match &mut self.backend {
            Backend::Winit(winit) => winit
                .backend
                .renderer()
                .import_dmabuf(&dmabuf, None)
                .map(|_| ())
                .map_err(|_| ImportError::Failed),
            Backend::Udev(udev) => udev
                .gpu_manager
                .single_renderer(&udev.primary_gpu)
                .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
                .map(|_| ())
                .map_err(|_| ImportError::Failed),
        }
    }
}
delegate_dmabuf!(State);
