// SPDX-License-Identifier: GPL-3.0-or-later

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        renderer::{ImportDma, Renderer, TextureFilter, gles::GlesRenderer},
    },
    delegate_dmabuf,
    output::Output,
    reexports::{calloop::LoopHandle, wayland_server::protocol::wl_surface::WlSurface},
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
};
use tracing::error;

use crate::{
    output::OutputMode,
    state::{Pinnacle, State, WithState},
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

    fn set_output_powered(
        &mut self,
        output: &Output,
        loop_handle: &LoopHandle<'static, State>,
        powered: bool,
    ) {
        match self {
            Backend::Winit(_) => (),
            Backend::Udev(udev) => udev.set_output_powered(output, loop_handle, powered),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.set_output_powered(output, powered),
        }
    }

    pub fn render_scheduled_outputs(&mut self, pinnacle: &mut Pinnacle) {
        if let Backend::Udev(udev) = self {
            for output in pinnacle
                .outputs
                .iter()
                .filter(|op| op.with_state(|state| state.enabled_global_id.is_some()))
                .cloned()
                .collect::<Vec<_>>()
            {
                udev.render_if_scheduled(pinnacle, &output);
            }
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

impl State {
    pub fn set_output_powered(&mut self, output: &Output, powered: bool) {
        self.backend
            .set_output_powered(output, &self.pinnacle.loop_handle, powered);
        self.pinnacle
            .output_power_management_state
            .mode_set(output, powered);
    }
}

impl Drop for State {
    fn drop(&mut self) {
        // Reset gamma when exiting
        if let Backend::Udev(udev) = &mut self.backend {
            for output in self.pinnacle.outputs.iter() {
                let _ = udev.set_gamma(output, None);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderResult {
    Submitted,
    NoDamage,
    Skipped,
}

pub trait BackendData: 'static {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);

    // INFO: only for udev in anvil, maybe shouldn't be a trait fn?
    fn early_import(&mut self, surface: &WlSurface);

    fn set_output_mode(&mut self, output: &Output, mode: OutputMode);
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

    fn set_output_mode(&mut self, output: &Output, mode: OutputMode) {
        match self {
            Backend::Winit(winit) => winit.set_output_mode(output, mode),
            Backend::Udev(udev) => udev.set_output_mode(output, mode),
            #[cfg(feature = "testing")]
            Backend::Dummy(dummy) => dummy.set_output_mode(output, mode),
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
