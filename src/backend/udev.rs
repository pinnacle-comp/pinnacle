// SPDX-License-Identifier: GPL-3.0-or-later

mod drm;
mod frame;
mod gamma;

use assert_matches::assert_matches;
pub use drm::drm_mode_from_modeinfo;
use frame::FrameClock;
use indexmap::IndexSet;
use wayland_backend::server::GlobalId;

use std::{collections::HashMap, mem, path::Path, time::Duration};

use anyhow::{Context, anyhow, ensure};
use drm::{create_drm_mode, refresh_interval};
use smithay::{
    backend::{
        SwapBuffersError,
        allocator::{
            Buffer, Fourcc,
            gbm::{GbmAllocator, GbmBuffer, GbmBufferFlags, GbmDevice},
        },
        drm::{
            DrmDevice, DrmDeviceFd, DrmEvent, DrmEventMetadata, DrmNode, DrmSurface, NodeType,
            compositor::{FrameFlags, PrimaryPlaneElement, RenderFrameResult},
            exporter::gbm::GbmFramebufferExporter,
            gbm::GbmFramebuffer,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
        },
        egl::{EGLDevice, EGLDisplay, context::ContextPriority},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            self, Bind, Blit, BufferType, ExportMem, ImportDma, ImportMemWl, Offscreen, Renderer,
            TextureFilter,
            damage::OutputDamageTracker,
            element::{self, Element, Id, surface::render_elements_from_surface_tree},
            gles::{GlesRenderbuffer, GlesRenderer},
            multigpu::{GpuManager, MultiRenderer, gbm::GbmGlesBackend},
            sync::SyncPoint,
            utils::{CommitCounter, DamageSet},
        },
        session::{self, Session, libseat::LibSeatSession},
        udev::{self, UdevBackend, UdevEvent},
    },
    desktop::utils::{OutputPresentationFeedback, surface_primary_scanout_output},
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            self, Dispatcher, Interest, LoopHandle, PostAction, RegistrationToken,
            generic::Generic,
            timer::{TimeoutAction, Timer},
        },
        drm::control::{ModeTypeFlags, connector, crtc},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_protocols::wp::{
            linux_dmabuf::zv1::server::zwp_linux_dmabuf_feedback_v1,
            presentation_time::server::wp_presentation_feedback,
        },
        wayland_server::{
            DisplayHandle,
            protocol::{wl_shm, wl_surface::WlSurface},
        },
    },
    utils::{DeviceFd, Rectangle, Transform},
    wayland::{
        dmabuf::{self, DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal},
        presentation::Refresh,
        shm::shm_format_to_fourcc,
    },
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};
use tracing::{debug, error, info, trace, warn};

use crate::{
    api::signal::Signal,
    backend::Backend,
    config::ConnectorSavedState,
    input::libinput::DeviceState,
    output::{BlankingState, OutputMode, OutputName},
    render::{
        CLEAR_COLOR, CLEAR_COLOR_LOCKED, OutputRenderElement, pointer::pointer_render_elements,
        take_presentation_feedback,
    },
    state::{FrameCallbackSequence, Pinnacle, State, WithState},
};

use super::{BackendData, UninitBackend};

const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];
const SUPPORTED_FORMATS_8BIT_ONLY: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];

/// A [`MultiRenderer`] that uses the [`GbmGlesBackend`].
pub type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

type UdevRenderFrameResult<'a> =
    RenderFrameResult<'a, GbmBuffer, GbmFramebuffer, OutputRenderElement<UdevRenderer<'a>>>;

/// Udev state attached to each [`Output`].
#[derive(Debug, PartialEq)]
struct UdevOutputData {
    /// The GPU node
    device_id: DrmNode,
    /// The [Crtc][crtc::Handle] the output is pushing to
    crtc: crtc::Handle,
}

// TODO: document desperately
pub struct Udev {
    pub session: LibSeatSession,
    udev_dispatcher: Dispatcher<'static, UdevBackend, State>,
    display_handle: DisplayHandle,
    pub primary_gpu: DrmNode,
    pub(super) gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    devices: HashMap<DrmNode, Device>,
    /// The global corresponding to the primary gpu
    dmabuf_global: Option<DmabufGlobal>,
    drm_global: Option<GlobalId>,

    pub(super) upscale_filter: TextureFilter,
    pub(super) downscale_filter: TextureFilter,
}

impl Backend {
    #[allow(dead_code)]
    fn udev(&self) -> &Udev {
        let Backend::Udev(udev) = self else { unreachable!() };
        udev
    }

    fn udev_mut(&mut self) -> &mut Udev {
        let Backend::Udev(udev) = self else { unreachable!() };
        udev
    }
}

impl Udev {
    pub(crate) fn try_new(display_handle: DisplayHandle) -> anyhow::Result<UninitBackend<Udev>> {
        // Initialize session
        let (session, notifier) = LibSeatSession::new()?;

        // Get the primary gpu
        let primary_gpu = udev::primary_gpu(session.seat())
            .context("unable to get primary gpu path")?
            .and_then(|x| {
                DrmNode::from_path(x)
                    .ok()?
                    .node_with_type(NodeType::Render)?
                    .ok()
            })
            .unwrap_or_else(|| {
                udev::all_gpus(session.seat())
                    .expect("failed to get gpu paths")
                    .into_iter()
                    .find_map(|x| DrmNode::from_path(x).ok())
                    .expect("No GPU!")
            });
        info!("Using {} as primary gpu", primary_gpu);

        let gpu_manager =
            GpuManager::new(GbmGlesBackend::with_context_priority(ContextPriority::High))?;

        // Initialize the udev backend
        let udev_backend = UdevBackend::new(session.seat())?;

        let udev_dispatcher = Dispatcher::new(udev_backend, move |event, _, state: &mut State| {
            let udev = state.backend.udev_mut();
            let pinnacle = &mut state.pinnacle;
            match event {
                // GPU connected
                UdevEvent::Added { device_id, path } => {
                    if let Err(err) = DrmNode::from_dev_id(device_id)
                        .context("failed to access drm node")
                        .and_then(|node| udev.device_added(pinnacle, node, &path))
                    {
                        error!("Skipping device {device_id}: {err}");
                    }
                }
                UdevEvent::Changed { device_id } => {
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        udev.device_changed(pinnacle, node)
                    }
                }
                // GPU disconnected
                UdevEvent::Removed { device_id } => {
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        udev.device_removed(pinnacle, node)
                    }
                }
            }
        });

        let mut udev = Udev {
            display_handle,
            udev_dispatcher,
            session,
            primary_gpu,
            gpu_manager,
            devices: HashMap::new(),
            dmabuf_global: None,
            drm_global: None,

            upscale_filter: TextureFilter::Linear,
            downscale_filter: TextureFilter::Linear,
        };

        Ok(UninitBackend {
            seat_name: udev.seat_name(),
            init: Box::new(move |pinnacle| {
                pinnacle
                    .loop_handle
                    .register_dispatcher(udev.udev_dispatcher.clone())?;

                let things = udev
                    .udev_dispatcher
                    .as_source_ref()
                    .device_list()
                    .map(|(id, path)| (id, path.to_path_buf()))
                    .collect::<Vec<_>>();

                // Create DrmNodes from already connected GPUs
                for (device_id, path) in things {
                    if let Err(err) = DrmNode::from_dev_id(device_id)
                        .context("failed to access drm node")
                        .and_then(|node| udev.device_added(pinnacle, node, &path))
                    {
                        error!("Skipping device {device_id}: {err}");
                    }
                }

                // Initialize libinput backend
                let mut libinput_context = Libinput::new_with_udev::<
                    LibinputSessionInterface<LibSeatSession>,
                >(udev.session.clone().into());
                libinput_context
                    .udev_assign_seat(pinnacle.seat.name())
                    .expect("failed to assign seat to libinput");
                let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

                // Bind all our objects that get driven by the event loop

                let insert_ret =
                    pinnacle
                        .loop_handle
                        .insert_source(libinput_backend, move |event, _, state| {
                            match &event {
                                smithay::backend::input::InputEvent::DeviceAdded { device } => {
                                    state
                                        .pinnacle
                                        .input_state
                                        .libinput_state
                                        .devices
                                        .insert(device.clone(), DeviceState::default());
                                    state
                                        .pinnacle
                                        .signal_state
                                        .input_device_added
                                        .signal(device);
                                }
                                smithay::backend::input::InputEvent::DeviceRemoved { device } => {
                                    state
                                        .pinnacle
                                        .input_state
                                        .libinput_state
                                        .devices
                                        .shift_remove(device);
                                }
                                _ => (),
                            }
                            state.process_input_event(event);
                        });

                if let Err(err) = insert_ret {
                    anyhow::bail!("Failed to insert libinput_backend into event loop: {err}");
                }

                pinnacle
                    .loop_handle
                    .insert_source(notifier, move |event, _, state| {
                        match event {
                            session::Event::PauseSession => {
                                let udev = state.backend.udev_mut();
                                libinput_context.suspend();
                                info!("pausing session");

                                for device in udev.devices.values_mut() {
                                    device.drm_output_manager.pause();
                                }
                            }
                            session::Event::ActivateSession => {
                                info!("resuming session");

                                if libinput_context.resume().is_err() {
                                    error!("Failed to resume libinput context");
                                }

                                let udev = state.backend.udev_mut();
                                let pinnacle = &mut state.pinnacle;

                                let (mut device_list, connected_devices, disconnected_devices) = {
                                    let device_list = udev
                                        .udev_dispatcher
                                        .as_source_ref()
                                        .device_list()
                                        .flat_map(|(id, path)| {
                                            Some((
                                                DrmNode::from_dev_id(id).ok()?,
                                                path.to_path_buf(),
                                            ))
                                        })
                                        .collect::<HashMap<_, _>>();

                                    let (connected_devices, disconnected_devices) =
                                        udev.devices.keys().copied().partition::<Vec<_>, _>(
                                            |node| device_list.contains_key(node),
                                        );

                                    (device_list, connected_devices, disconnected_devices)
                                };

                                for node in disconnected_devices {
                                    device_list.remove(&node);
                                    udev.device_removed(pinnacle, node);
                                }

                                for node in connected_devices {
                                    device_list.remove(&node);

                                    // INFO: see if this can be moved below udev.device_changed
                                    {
                                        let Some(device) = udev.devices.get_mut(&node) else {
                                            unreachable!();
                                        };

                                        if let Err(err) =
                                            device.drm_output_manager.lock().activate(true)
                                        {
                                            error!("Error activating DRM device: {err}");
                                        }
                                    }

                                    udev.device_changed(pinnacle, node);

                                    let Some(device) = udev.devices.get_mut(&node) else {
                                        unreachable!();
                                    };

                                    // Apply pending gammas
                                    //
                                    // Also welcome to some really doodoo code

                                    for (crtc, surface) in device.surfaces.iter_mut() {
                                        match mem::take(&mut surface.pending_gamma_change) {
                                            PendingGammaChange::Idle => {
                                                debug!("Restoring from previous gamma");
                                                if let Err(err) = Udev::set_gamma_internal(
                                                    device.drm_output_manager.device(),
                                                    crtc,
                                                    surface.previous_gamma.clone(),
                                                ) {
                                                    warn!("Failed to reset gamma: {err}");
                                                    surface.previous_gamma = None;
                                                }
                                            }
                                            PendingGammaChange::Restore => {
                                                debug!("Restoring to original gamma");
                                                if let Err(err) = Udev::set_gamma_internal(
                                                    device.drm_output_manager.device(),
                                                    crtc,
                                                    None::<[&[u16]; 3]>,
                                                ) {
                                                    warn!("Failed to reset gamma: {err}");
                                                }
                                                surface.previous_gamma = None;
                                            }
                                            PendingGammaChange::Change(gamma) => {
                                                debug!("Changing to pending gamma");
                                                match Udev::set_gamma_internal(
                                                    device.drm_output_manager.device(),
                                                    crtc,
                                                    Some([&gamma[0], &gamma[1], &gamma[2]]),
                                                ) {
                                                    Ok(()) => {
                                                        surface.previous_gamma = Some(gamma);
                                                    }
                                                    Err(err) => {
                                                        warn!("Failed to set pending gamma: {err}");
                                                        surface.previous_gamma = None;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Newly connected devices
                                for (node, path) in device_list.into_iter() {
                                    if let Err(err) = state.backend.udev_mut().device_added(
                                        &mut state.pinnacle,
                                        node,
                                        &path,
                                    ) {
                                        error!("Error adding device: {err}");
                                    }
                                }

                                for output in
                                    state.pinnacle.space.outputs().cloned().collect::<Vec<_>>()
                                {
                                    state.schedule_render(&output);
                                }
                            }
                        }
                    })
                    .expect("failed to insert libinput notifier into event loop");

                pinnacle.shm_state.update_formats(
                    udev.gpu_manager
                        .single_renderer(&primary_gpu)?
                        .shm_formats(),
                );

                Ok(udev)
            }),
        })
    }

    /// Schedule a new render that will cause the compositor to redraw everything.
    pub fn schedule_render(&mut self, output: &Output) {
        let _span = tracy_client::span!("Udev::schedule_render");

        let Some(surface) = render_surface_for_output(output, &mut self.devices) else {
            debug!("no render surface on output {}", output.name());
            return;
        };

        let old_state = mem::take(&mut surface.render_state);

        surface.render_state = match old_state {
            RenderState::Idle => RenderState::Scheduled,

            value @ (RenderState::Scheduled
            | RenderState::WaitingForEstimatedVblankAndScheduled(_)) => value,

            RenderState::WaitingForVblank { .. } => RenderState::WaitingForVblank {
                render_needed: true,
            },

            RenderState::WaitingForEstimatedVblank(token) => {
                RenderState::WaitingForEstimatedVblankAndScheduled(token)
            }
        };
    }

    pub(super) fn set_output_powered(
        &mut self,
        output: &Output,
        loop_handle: &LoopHandle<'static, State>,
        powered: bool,
    ) {
        let _span = tracy_client::span!("Udev::set_output_powered");

        let UdevOutputData { device_id, crtc } =
            output.user_data().get::<UdevOutputData>().unwrap();

        let Some(device) = self.devices.get_mut(device_id) else {
            return;
        };

        if powered {
            output.with_state_mut(|state| state.powered = true);
        } else {
            output.with_state_mut(|state| state.powered = false);

            if let Err(err) = device
                .surfaces
                .get_mut(crtc)
                .unwrap()
                .drm_output
                .with_compositor(|compositor| compositor.clear())
            {
                warn!("Failed to clear compositor state on crtc {crtc:?}: {err}");
            }

            if let Some(surface) = render_surface_for_output(output, &mut self.devices)
                && let RenderState::WaitingForEstimatedVblankAndScheduled(token)
                | RenderState::WaitingForEstimatedVblank(token) =
                    mem::take(&mut surface.render_state)
            {
                loop_handle.remove(token);
            }
        }
    }
}

impl State {
    /// Switch the tty.
    ///
    /// Does nothing when called on the winit backend.
    pub fn switch_vt(&mut self, vt: i32) {
        if let Backend::Udev(udev) = &mut self.backend {
            info!("Switching to vt {vt}");
            if let Err(err) = udev.session.change_vt(vt) {
                error!("Failed to switch to vt {vt}: {err}");
            }
        }
    }
}

impl BackendData for Udev {
    fn seat_name(&self) -> String {
        self.session.seat()
    }

    fn reset_buffers(&mut self, output: &Output) {
        let _span = tracy_client::span!("Udev: BackendData::reset_buffers");

        if let Some(id) = output.user_data().get::<UdevOutputData>()
            && let Some(gpu) = self.devices.get_mut(&id.device_id)
            && let Some(surface) = gpu.surfaces.get_mut(&id.crtc)
        {
            surface.drm_output.reset_buffers();
        }
    }

    fn early_import(&mut self, surface: &WlSurface) {
        let _span = tracy_client::span!("Udev: BackendData::early_import");

        if let Err(err) = self.gpu_manager.early_import(self.primary_gpu, surface) {
            warn!("early buffer import failed: {}", err);
        }
    }

    fn set_output_mode(&mut self, output: &Output, mode: OutputMode) {
        let _span = tracy_client::span!("Udev: BackendData::set_output_mode");

        let drm_mode = self
            .devices
            .iter()
            .find_map(|(_, device)| {
                device
                    .drm_scanner
                    .crtcs()
                    .find(|(_, handle)| {
                        output
                            .user_data()
                            .get::<UdevOutputData>()
                            .is_some_and(|data| &data.crtc == handle)
                    })
                    .and_then(|(info, _)| {
                        info.modes()
                            .iter()
                            .find(|m| smithay::output::Mode::from(**m) == mode.into())
                    })
                    .copied()
            })
            .unwrap_or_else(|| {
                info!("Unknown mode for {}, creating new one", output.name());
                match mode {
                    OutputMode::Smithay(mode) => {
                        create_drm_mode(mode.size.w, mode.size.h, Some(mode.refresh as u32))
                    }
                    OutputMode::Drm(mode) => mode,
                }
            });

        if let Some(render_surface) = render_surface_for_output(output, &mut self.devices)
            && let Ok(mut renderer) = self.gpu_manager.single_renderer(&self.primary_gpu)
        {
            match render_surface.drm_output.use_mode(
                drm_mode,
                &mut renderer,
                &DrmOutputRenderElements::<_, OutputRenderElement<_>>::default(),
            ) {
                Ok(()) => {
                    let mode = smithay::output::Mode::from(mode);
                    info!(
                        "Set {}'s mode to {}x{}@{:.3}Hz",
                        output.name(),
                        mode.size.w,
                        mode.size.h,
                        mode.refresh as f64 / 1000.0
                    );
                    output.change_current_state(Some(mode), None, None, None);
                    output.with_state_mut(|state| {
                        // TODO: push or no?
                        if !state.modes.contains(&mode) {
                            state.modes.push(mode);
                        }
                    });
                }
                Err(err) => warn!("Failed to set output mode for {}: {err}", output.name()),
            }
        }
    }
}

// TODO: document desperately
struct Device {
    surfaces: HashMap<crtc::Handle, RenderSurface>,
    drm_output_manager: DrmOutputManager<
        GbmAllocator<DrmDeviceFd>,
        GbmFramebufferExporter<DrmDeviceFd>,
        Option<OutputPresentationFeedback>,
        DrmDeviceFd,
    >,
    drm_scanner: DrmScanner,
    render_node: DrmNode,
    registration_token: RegistrationToken,
}

fn get_surface_dmabuf_feedback(
    primary_gpu: DrmNode,
    render_node: DrmNode,
    gpu_manager: &mut GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    surface: &DrmSurface,
) -> Option<SurfaceDmabufFeedback> {
    let _span = tracy_client::span!("get_surface_dmabuf_feedback");

    let primary_formats = gpu_manager
        .single_renderer(&primary_gpu)
        .ok()?
        .dmabuf_formats();

    let render_formats = gpu_manager
        .single_renderer(&render_node)
        .ok()?
        .dmabuf_formats();

    let all_render_formats = primary_formats
        .iter()
        .chain(render_formats.iter())
        .copied()
        .collect::<IndexSet<_>>();

    let planes = surface.planes().clone();

    // We limit the scan-out trache to formats we can also render from
    // so that there is always a fallback render path available in case
    // the supplied buffer can not be scanned out directly
    let planes_formats = planes
        .primary
        .into_iter()
        .flat_map(|p| p.formats)
        .chain(planes.overlay.into_iter().flat_map(|p| p.formats))
        .collect::<IndexSet<_>>()
        .intersection(&all_render_formats)
        .copied()
        .collect::<Vec<_>>();

    let builder = DmabufFeedbackBuilder::new(primary_gpu.dev_id(), primary_formats);
    let render_feedback = builder
        .clone()
        .add_preference_tranche(render_node.dev_id(), None, render_formats.clone())
        .build()
        .ok()?; // INFO: this is an unwrap in Anvil, does it matter?

    let scanout_feedback = builder
        .add_preference_tranche(
            surface.device_fd().dev_id().ok()?, // INFO: this is an unwrap in Anvil, does it matter?
            Some(zwp_linux_dmabuf_feedback_v1::TrancheFlags::Scanout),
            planes_formats,
        )
        .add_preference_tranche(render_node.dev_id(), None, render_formats)
        .build()
        .ok()?; // INFO: this is an unwrap in Anvil, does it matter?

    Some(SurfaceDmabufFeedback {
        render_feedback,
        scanout_feedback,
    })
}

#[derive(Debug)]
pub struct SurfaceDmabufFeedback {
    pub render_feedback: DmabufFeedback,
    pub scanout_feedback: DmabufFeedback,
}

/// The state of a [`RenderSurface`].
#[derive(Debug, Default)]
enum RenderState {
    /// No render is scheduled.
    #[default]
    Idle,
    /// A render is scheduled to happen at the end of the current event loop cycle.
    Scheduled,
    /// A frame was rendered and we are waiting for vblank.
    WaitingForVblank {
        /// A render was scheduled while waiting for vblank.
        /// In this case, another render will be scheduled once vblank happens.
        render_needed: bool,
    },
    /// A frame caused no damage, but we'll still wait as if it did to prevent busy loops.
    WaitingForEstimatedVblank(RegistrationToken),
    /// A render was scheduled during a wait for estimated vblank.
    WaitingForEstimatedVblankAndScheduled(RegistrationToken),
}

/// Render surface for an output.
struct RenderSurface {
    /// The node from `connector_connected`.
    device_id: DrmNode,
    /// The node rendering to the screen? idk
    ///
    /// If this is equal to the primary gpu node then it does the rendering operations.
    /// If it's not it is the node the composited buffer ends up on.
    render_node: DrmNode,
    drm_output: DrmOutput<
        GbmAllocator<DrmDeviceFd>,
        GbmFramebufferExporter<DrmDeviceFd>,
        Option<OutputPresentationFeedback>,
        DrmDeviceFd,
    >,
    dmabuf_feedback: Option<SurfaceDmabufFeedback>,
    render_state: RenderState,
    screencopy_commit_state: ScreencopyCommitState,

    previous_gamma: Option<[Box<[u16]>; 3]>,
    pending_gamma_change: PendingGammaChange,

    frame_clock: FrameClock,
    frame_callback_sequence: FrameCallbackSequence,
}

#[derive(Debug, Clone, Default)]
enum PendingGammaChange {
    /// No pending gamma
    #[default]
    Idle,
    /// Restore the original gamma
    Restore,
    /// Change the gamma
    Change([Box<[u16]>; 3]),
}

#[derive(Default, Debug, Clone, Copy)]
struct ScreencopyCommitState {
    primary_plane_swapchain: CommitCounter,
    primary_plane_element: CommitCounter,
    cursor: CommitCounter,
}

impl Udev {
    pub fn renderer(&mut self) -> anyhow::Result<UdevRenderer<'_>> {
        Ok(self.gpu_manager.single_renderer(&self.primary_gpu)?)
    }

    /// A GPU was plugged in.
    fn device_added(
        &mut self,
        pinnacle: &mut Pinnacle,
        node: DrmNode,
        path: &Path,
    ) -> anyhow::Result<()> {
        debug!(?node, ?path, "Udev::device_added");

        // Try to open the device
        let fd = self
            .session
            .open(
                path,
                OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
            )
            .context("failed to open device with libseat")?;

        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (drm, notifier) =
            DrmDevice::new(fd.clone(), true).context("failed to init drm device")?;
        let gbm = GbmDevice::new(fd).context("failed to init gbm device")?;

        let registration_token = pinnacle
            .loop_handle
            .insert_source(notifier, move |event, metadata, state| {
                let metadata = metadata.expect("vblank events must have metadata");
                match event {
                    DrmEvent::VBlank(crtc) => {
                        state.backend.udev_mut().on_vblank(
                            &mut state.pinnacle,
                            node,
                            crtc,
                            metadata,
                        );
                    }
                    DrmEvent::Error(error) => {
                        error!("{:?}", error);
                    }
                }
            })
            .expect("failed to insert drm notifier into event loop");

        // INFO: Anvil changes this as of c21ff35, figure that out
        // SAFETY: no clue lol just copied this from anvil
        let render_node = EGLDevice::device_for_display(&unsafe {
            EGLDisplay::new(gbm.clone()).expect("failed to create EGLDisplay")
        })
        .ok()
        .and_then(|x| x.try_get_render_node().ok().flatten())
        .unwrap_or(node);

        self.gpu_manager
            .as_mut()
            .add_node(render_node, gbm.clone())
            .context("failed to add device to GpuManager")?;

        if render_node == self.primary_gpu {
            let renderer = self.gpu_manager.single_renderer(&render_node)?;

            let (dmabuf_global, drm_global_id) = pinnacle
                .init_hardware_accel(render_node, renderer.dmabuf_formats())
                .inspect_err(|err| {
                    error!("Failed to initialize EGL hardware acceleration: {err}");
                })?;

            assert!(self.dmabuf_global.replace(dmabuf_global).is_none());
            assert!(self.drm_global.replace(drm_global_id).is_none());

            // Update the per drm surface dmabuf feedback
            for device in self.devices.values_mut() {
                for surface in device.surfaces.values_mut() {
                    let dmabuf_feedback = surface.drm_output.with_compositor(|compositor| {
                        get_surface_dmabuf_feedback(
                            render_node,
                            surface.render_node,
                            &mut self.gpu_manager,
                            compositor.surface(),
                        )
                    });

                    if let Some(dmabuf_feedback) = dmabuf_feedback {
                        surface.dmabuf_feedback.replace(dmabuf_feedback);
                    }
                }
            }
        }

        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let color_formats = if std::env::var("PINNACLE_DISABLE_10BIT").is_ok() {
            SUPPORTED_FORMATS_8BIT_ONLY
        } else {
            SUPPORTED_FORMATS
        };

        let mut renderer = self.gpu_manager.single_renderer(&render_node).unwrap();
        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .clone();

        let drm_output_manager = DrmOutputManager::new(
            drm,
            allocator,
            GbmFramebufferExporter::new(gbm.clone(), render_node.into()),
            Some(gbm),
            color_formats.iter().copied(),
            render_formats,
        );

        self.devices.insert(
            node,
            Device {
                registration_token,
                drm_output_manager,
                drm_scanner: DrmScanner::new(),
                render_node,
                surfaces: HashMap::new(),
            },
        );

        self.device_changed(pinnacle, node);

        Ok(())
    }

    /// A display was plugged in.
    fn connector_connected(
        &mut self,
        pinnacle: &mut Pinnacle,
        node: DrmNode,
        connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        debug!(?node, ?connector, ?crtc, "Udev::connector_connected");

        let Some(device) = self.devices.get_mut(&node) else {
            warn!(?node, "Device disappeared");
            return;
        };

        let mut renderer = self
            .gpu_manager
            .single_renderer(&device.render_node)
            .expect("failed to get primary gpu MultiRenderer");

        info!(
            ?crtc,
            "Trying to setup connector {:?}-{}",
            connector.interface(),
            connector.interface_id(),
        );

        let mode_id = connector
            .modes()
            .iter()
            .position(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
            .unwrap_or(0);

        let drm_mode = connector.modes()[mode_id];
        let smithay_mode = smithay::output::Mode::from(drm_mode);

        let drm_device = device.drm_output_manager.device_mut();

        let surface = match drm_device.create_surface(crtc, drm_mode, &[connector.handle()]) {
            Ok(surface) => surface,
            Err(err) => {
                warn!("Failed to create drm surface: {}", err);
                return;
            }
        };

        let output_name = format!(
            "{}-{}",
            connector.interface().as_str(),
            connector.interface_id()
        );

        let display_info =
            smithay_drm_extras::display_info::for_connector(drm_device, connector.handle());

        let (make, model, serial) = display_info
            .map(|info| {
                (
                    info.make().unwrap_or("Unknown".into()),
                    info.model().unwrap_or("Unknown".into()),
                    info.serial().unwrap_or("Unknown".into()),
                )
            })
            .unwrap_or_else(|| ("Unknown".into(), "Unknown".into(), "Unknown".into()));

        let (phys_w, phys_h) = connector.size().unwrap_or_default();

        let output = Output::new(
            output_name,
            PhysicalProperties {
                size: (phys_w as i32, phys_h as i32).into(),
                subpixel: Subpixel::from(connector.subpixel()),
                make,
                model,
                serial_number: serial,
            },
        );
        let global = output.create_global::<State>(&self.display_handle);
        output.with_state_mut(|state| state.enabled_global_id = Some(global));

        pinnacle.outputs.push(output.clone());
        pinnacle.output_focus_stack.add_to_end(output.clone());

        output.with_state_mut(|state| {
            state.debug_damage_tracker = OutputDamageTracker::from_output(&output);
        });

        output.set_preferred(smithay_mode);

        let modes = connector
            .modes()
            .iter()
            .cloned()
            .map(smithay::output::Mode::from)
            .collect::<Vec<_>>();
        output.with_state_mut(|state| state.modes = modes);

        pinnacle
            .output_management_manager_state
            .add_head::<State>(&output);

        let x = pinnacle.space.outputs().fold(0, |acc, o| {
            let Some(geo) = pinnacle.space.output_geometry(o) else {
                unreachable!()
            };
            acc + geo.size.w
        });
        let position = (x, 0).into();

        output.change_current_state(Some(smithay_mode), None, None, Some(position));

        output.user_data().insert_if_missing(|| UdevOutputData {
            crtc,
            device_id: node,
        });

        let drm_output = {
            let planes = surface.planes().clone();

            match device.drm_output_manager.lock().initialize_output(
                crtc,
                drm_mode,
                &[connector.handle()],
                &output,
                Some(planes),
                &mut renderer,
                &DrmOutputRenderElements::<_, OutputRenderElement<_>>::default(),
            ) {
                Ok(drm_output) => drm_output,
                Err(err) => {
                    warn!("Failed to create drm compositor: {}", err);
                    return;
                }
            }
        };

        let dmabuf_feedback = drm_output.with_compositor(|compositor| {
            get_surface_dmabuf_feedback(
                self.primary_gpu,
                device.render_node,
                &mut self.gpu_manager,
                compositor.surface(),
            )
        });

        let surface = RenderSurface {
            device_id: node,
            render_node: device.render_node,
            drm_output,
            dmabuf_feedback,
            render_state: RenderState::Idle,
            screencopy_commit_state: ScreencopyCommitState::default(),
            previous_gamma: None,
            pending_gamma_change: PendingGammaChange::Idle,
            frame_clock: FrameClock::new(Some(refresh_interval(drm_mode))),
            frame_callback_sequence: FrameCallbackSequence::default(),
        };

        device.surfaces.insert(crtc, surface);

        pinnacle.change_output_state(
            self,
            &output,
            Some(OutputMode::Smithay(smithay_mode)),
            None,
            None,
            Some(position),
        );

        // If there is saved connector state, the connector was previously plugged in.
        // In this case, restore its tags and location.
        // TODO: instead of checking the connector, check the monitor's edid info instead
        if let Some(saved_state) = pinnacle
            .config
            .connector_saved_states
            .get(&OutputName(output.name()))
        {
            let ConnectorSavedState {
                loc,
                tags,
                scale,
                powered,
            } = saved_state.clone();

            output.with_state_mut(|state| state.tags.clone_from(&tags));
            pinnacle.change_output_state(self, &output, None, None, scale, Some(loc));
            if let Some(powered) = powered {
                self.set_output_powered(&output, &pinnacle.loop_handle, powered);
            }
        } else {
            pinnacle.signal_state.output_setup.signal(&output);
        }

        pinnacle.signal_state.output_connect.signal(&output);

        pinnacle.output_management_manager_state.update::<State>();
    }

    /// A display was unplugged.
    fn connector_disconnected(
        &mut self,
        pinnacle: &mut Pinnacle,
        node: DrmNode,
        crtc: crtc::Handle,
    ) {
        debug!(?node, ?crtc, "Udev::connector_disconnected");

        let Some(device) = self.devices.get_mut(&node) else {
            warn!(?node, "Device disappeared");
            return;
        };

        device.surfaces.remove(&crtc);

        let output = pinnacle
            .outputs
            .iter()
            .find(|o| {
                o.user_data()
                    .get::<UdevOutputData>()
                    .map(|id| id.device_id == node && id.crtc == crtc)
                    .unwrap_or(false)
            })
            .cloned();

        if let Some(output) = output {
            pinnacle.remove_output(&output);
        }
    }

    fn device_changed(&mut self, pinnacle: &mut Pinnacle, node: DrmNode) {
        debug!(?node, "Udev::device_changed");

        let Some(device) = self.devices.get_mut(&node) else {
            warn!(?node, "Device disappeared");
            return;
        };

        let drm_scan_result = match device
            .drm_scanner
            .scan_connectors(device.drm_output_manager.device())
        {
            Ok(scan_result) => scan_result,
            Err(err) => {
                error!("Failed to scan drm connectors: {err}");
                return;
            }
        };

        for event in drm_scan_result {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_connected(pinnacle, node, connector, crtc);
                }
                DrmScanEvent::Disconnected {
                    connector: _,
                    crtc: Some(crtc),
                } => {
                    self.connector_disconnected(pinnacle, node, crtc);
                }
                _ => {}
            }
        }
    }

    /// A GPU was unplugged.
    fn device_removed(&mut self, pinnacle: &mut Pinnacle, node: DrmNode) {
        debug!(?node, "Udev::device_removed");

        let Some(device) = self.devices.get(&node) else {
            warn!(?node, "Device disappeared");
            return;
        };

        let crtcs = device
            .drm_scanner
            .crtcs()
            .map(|(_info, crtc)| crtc)
            .collect::<Vec<_>>();

        for crtc in crtcs {
            self.connector_disconnected(pinnacle, node, crtc);
        }

        debug!("Surfaces dropped");

        let Some(device) = self.devices.remove(&node) else {
            unreachable!()
        };

        self.gpu_manager.as_mut().remove_node(&device.render_node);

        pinnacle.loop_handle.remove(device.registration_token);

        if node == self.primary_gpu {
            if let Some(dmabuf_global) = self.dmabuf_global.take() {
                pinnacle
                    .dmabuf_state
                    .disable_global::<State>(&pinnacle.display_handle, &dmabuf_global);
                // Niri waits 10 seconds to destroy the global
                pinnacle
                    .loop_handle
                    .insert_source(
                        Timer::from_duration(Duration::from_secs(10)),
                        move |_, _, state| {
                            state.pinnacle.dmabuf_state.destroy_global::<State>(
                                &state.pinnacle.display_handle,
                                dmabuf_global,
                            );
                            TimeoutAction::Drop
                        },
                    )
                    .unwrap();

                for device in self.devices.values_mut() {
                    for surface in device.surfaces.values_mut() {
                        surface.dmabuf_feedback = None;
                    }
                }
            } else {
                error!("Could not remove dmabuf global because it was missing");
            }

            if let Some(drm_global_id) = self.drm_global.take() {
                pinnacle
                    .display_handle
                    .remove_global::<State>(drm_global_id);
            } else {
                error!("Could not remove drm global because it was missing");
            }
        }
    }

    fn on_vblank(
        &mut self,
        pinnacle: &mut Pinnacle,
        dev_id: DrmNode,
        crtc: crtc::Handle,
        metadata: DrmEventMetadata,
    ) {
        let span = tracy_client::span!("Udev::on_vblank");

        let Some(surface) = self
            .devices
            .get_mut(&dev_id)
            .and_then(|device| device.surfaces.get_mut(&crtc))
        else {
            return;
        };

        let output = if let Some(output) = pinnacle.outputs.iter().find(|o| {
            let udev_op_data = o.user_data().get::<UdevOutputData>();
            udev_op_data
                .is_some_and(|data| data.device_id == surface.device_id && data.crtc == crtc)
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        span.emit_text(&output.name());

        let presentation_time = match metadata.time {
            smithay::backend::drm::DrmEventTime::Monotonic(tp) => tp,
            smithay::backend::drm::DrmEventTime::Realtime(_) => {
                // Not supported

                // This value will be ignored in the frame clock code
                Duration::ZERO
            }
        };

        match surface
            .drm_output
            .frame_submitted()
            .map_err(SwapBuffersError::from)
        {
            Ok(user_data) => {
                if let Some(mut feedback) = user_data.flatten() {
                    let seq = metadata.sequence as u64;

                    let mut flags = wp_presentation_feedback::Kind::Vsync
                        | wp_presentation_feedback::Kind::HwCompletion;

                    let time: Duration = if presentation_time.is_zero() {
                        pinnacle.clock.now().into()
                    } else {
                        flags.insert(wp_presentation_feedback::Kind::HwClock);
                        presentation_time
                    };

                    let refresh = surface
                        .frame_clock
                        .refresh_interval()
                        .map(match surface.frame_clock.vrr() {
                            true => Refresh::Variable,
                            false => Refresh::Fixed,
                        })
                        .unwrap_or(Refresh::Unknown);

                    feedback.presented::<_, smithay::utils::Monotonic>(time, refresh, seq, flags);
                }

                output.with_state_mut(|state| {
                    if let BlankingState::Blanking = state.blanking_state {
                        debug!("Output {} blanked", output.name());
                        state.blanking_state = BlankingState::Blanked;
                    }
                })
            }
            Err(err) => {
                warn!("Error during rendering: {err:?}");
                if let SwapBuffersError::ContextLost(err) = err {
                    panic!("Rendering loop lost: {err}")
                }
            }
        };

        surface.frame_clock.presented(presentation_time);

        let render_needed = match mem::take(&mut surface.render_state) {
            RenderState::WaitingForVblank { render_needed } => render_needed,
            state => {
                debug!("vblank happened but render state was {state:?}",);
                return;
            }
        };

        if render_needed || pinnacle.cursor_state.is_current_cursor_animated() {
            self.schedule_render(&output);
        } else {
            pinnacle.send_frame_callbacks(&output, Some(surface.frame_callback_sequence));
        }
    }

    pub(super) fn render_if_scheduled(&mut self, pinnacle: &mut Pinnacle, output: &Output) {
        let span = tracy_client::span!("Udev::render_if_scheduled");
        span.emit_text(&output.name());

        let Some(surface) = render_surface_for_output(output, &mut self.devices) else {
            return;
        };

        if matches!(
            surface.render_state,
            RenderState::Scheduled | RenderState::WaitingForEstimatedVblankAndScheduled(_)
        ) {
            self.render_surface(pinnacle, output);
        }
    }

    /// Render to the [`RenderSurface`] associated with the given `output`.
    fn render_surface(&mut self, pinnacle: &mut Pinnacle, output: &Output) {
        let span = tracy_client::span!("Udev::render_surface");
        span.emit_text(&output.name());

        let UdevOutputData { device_id, .. } = output.user_data().get().unwrap();
        let is_active = self
            .devices
            .get(device_id)
            .map(|device| device.drm_output_manager.device().is_active())
            .unwrap_or_default();

        let Some(surface) = render_surface_for_output(output, &mut self.devices) else {
            return;
        };

        let make_idle = |render_state: &mut RenderState,
                         loop_handle: &LoopHandle<'static, State>| {
            if let RenderState::WaitingForEstimatedVblankAndScheduled(token)
            | RenderState::WaitingForEstimatedVblank(token) = mem::take(render_state)
            {
                loop_handle.remove(token);
            }
        };

        if !is_active {
            warn!("Device is inactive");
            make_idle(&mut surface.render_state, &pinnacle.loop_handle);
            return;
        }

        if !pinnacle.outputs.contains(output) {
            make_idle(&mut surface.render_state, &pinnacle.loop_handle);
            return;
        }

        // TODO: possibly lift this out and make it so that scheduling a render
        // does nothing on powered off outputs
        if output.with_state(|state| !state.powered) {
            make_idle(&mut surface.render_state, &pinnacle.loop_handle);
            return;
        }

        let Some(output_geo) = pinnacle.space.output_geometry(output) else {
            make_idle(&mut surface.render_state, &pinnacle.loop_handle);
            return;
        };

        assert_matches!(
            surface.render_state,
            RenderState::Scheduled | RenderState::WaitingForEstimatedVblankAndScheduled(_)
        );

        let time_to_next_presentation = surface
            .frame_clock
            .time_to_next_presentation(&pinnacle.clock);

        let render_node = surface.render_node;
        let primary_gpu = self.primary_gpu;
        let mut renderer = if primary_gpu == render_node {
            self.gpu_manager.single_renderer(&render_node)
        } else {
            let format = surface.drm_output.format();
            self.gpu_manager
                .renderer(&primary_gpu, &render_node, format)
        }
        .expect("failed to create MultiRenderer");

        let _ = renderer.upscale_filter(self.upscale_filter);
        let _ = renderer.downscale_filter(self.downscale_filter);

        let pointer_location = pinnacle
            .seat
            .get_pointer()
            .map(|ptr| ptr.current_location())
            .unwrap_or((0.0, 0.0).into());

        let mut output_render_elements = Vec::new();

        let should_blank = pinnacle.lock_state.is_locking()
            || (pinnacle.lock_state.is_locked()
                && output.with_state(|state| state.lock_surface.is_none()));

        let scale = output.current_scale().fractional_scale();

        let (pointer_render_elements, cursor_ids) = pointer_render_elements(
            pointer_location - output_geo.loc.to_f64(),
            scale,
            &mut renderer,
            &mut pinnacle.cursor_state,
            &pinnacle.clock,
        );
        output_render_elements.extend(
            pointer_render_elements
                .into_iter()
                .map(OutputRenderElement::from),
        );

        if should_blank {
            output.with_state_mut(|state| {
                if let BlankingState::NotBlanked = state.blanking_state {
                    debug!("Blanking output {} for session lock", output.name());
                    state.blanking_state = BlankingState::Blanking;
                }
            });
        } else if pinnacle.lock_state.is_locked() {
            if let Some(lock_surface) = output.with_state(|state| state.lock_surface.clone()) {
                let elems = render_elements_from_surface_tree(
                    &mut renderer,
                    lock_surface.wl_surface(),
                    (0, 0),
                    output.current_scale().fractional_scale(),
                    1.0,
                    element::Kind::Unspecified,
                );

                output_render_elements.extend(elems);
            }
        } else {
            output_render_elements.extend(crate::render::output_render_elements(
                output,
                &mut renderer,
                &pinnacle.space,
                &pinnacle.z_index_stack,
            ));
        }

        if pinnacle.config.debug.visualize_opaque_regions {
            crate::render::util::render_opaque_regions(
                &mut output_render_elements,
                smithay::utils::Scale::from(output.current_scale().fractional_scale()),
            );
        }

        if pinnacle.config.debug.visualize_damage {
            let damage_elements = output.with_state_mut(|state| {
                crate::render::util::render_damage_from_elements(
                    &mut state.debug_damage_tracker,
                    &output_render_elements,
                    [0.3, 0.0, 0.0, 0.3].into(),
                )
            });
            output_render_elements = damage_elements
                .into_iter()
                .map(From::from)
                .chain(output_render_elements)
                .collect();
        }

        let clear_color = if pinnacle.lock_state.is_unlocked() {
            CLEAR_COLOR
        } else {
            CLEAR_COLOR_LOCKED
        };

        // No overlay planes cuz they wonk
        let mut frame_flags =
            FrameFlags::ALLOW_PRIMARY_PLANE_SCANOUT_ANY | FrameFlags::ALLOW_CURSOR_PLANE_SCANOUT;

        if pinnacle.config.debug.disable_cursor_plane_scanout {
            frame_flags.remove(FrameFlags::ALLOW_CURSOR_PLANE_SCANOUT);
        }

        if surface.frame_clock.vrr()
            && let Some(time_since_last_presentation) = surface
                .frame_clock
                .time_since_last_presentation(&pinnacle.clock)
        {
            // We want to skip cursor-only updates so moving the mouse doesn't jump the refresh
            // rate to the max, which would cause stuttering in games. We're doing this only
            // when vrr is on and the cursor is above a fullscreen window.
            //
            // However, this would cause the cursor to freeze if the window doesn't refresh.
            // Therefore we're forcing the cursor to refresh at at least 24 fps.
            //
            // TODO: This is probably not the best behavior for videos. We should use content-type
            // to improve that.

            const _24_FPS: Duration = Duration::from_nanos(1_000_000_000 / 24);

            let too_long_since_last_present =
                time_to_next_presentation + time_since_last_presentation > _24_FPS;
            let window_under = pinnacle.space.element_under(
                pinnacle
                    .seat
                    .get_pointer()
                    .unwrap()
                    .current_location()
                    .to_i32_round(),
            );
            let cursor_over_fs_window = window_under
                .is_some_and(|(win, _)| win.with_state(|state| state.layout_mode.is_fullscreen()));
            if !too_long_since_last_present && cursor_over_fs_window {
                // FIXME: With a non-1 scale, the cursor no longer resides on the cursor plane,
                // making this useless

                frame_flags |= FrameFlags::SKIP_CURSOR_ONLY_UPDATES;
            }
        }

        let render_frame_result = surface.drm_output.render_frame(
            &mut renderer,
            &output_render_elements,
            clear_color,
            frame_flags,
        );

        let failed = match render_frame_result {
            Ok(res) => {
                if res.needs_sync()
                    && let PrimaryPlaneElement::Swapchain(element) = &res.primary_element
                    && let Err(err) = element.sync.wait()
                {
                    warn!("Failed to wait for sync point: {err}");
                }

                if pinnacle.lock_state.is_unlocked() {
                    handle_pending_screencopy(
                        &mut renderer,
                        output,
                        surface,
                        &res,
                        &pinnacle.loop_handle,
                        cursor_ids,
                    );
                }

                pinnacle.update_primary_scanout_output(output, &res.states);

                if let Some(dmabuf_feedback) = surface.dmabuf_feedback.as_ref() {
                    pinnacle.send_dmabuf_feedback(output, dmabuf_feedback, &res.states);
                }

                let rendered = !res.is_empty;

                if rendered {
                    let output_presentation_feedback =
                        take_presentation_feedback(output, &pinnacle.space, &res.states);

                    match surface
                        .drm_output
                        .queue_frame(Some(output_presentation_feedback))
                    {
                        Ok(()) => {
                            let new_state = RenderState::WaitingForVblank {
                                render_needed: false,
                            };

                            match mem::replace(&mut surface.render_state, new_state) {
                                RenderState::Idle => unreachable!(),
                                RenderState::Scheduled => (),
                                RenderState::WaitingForVblank { .. } => unreachable!(),
                                RenderState::WaitingForEstimatedVblank(_) => unreachable!(),
                                RenderState::WaitingForEstimatedVblankAndScheduled(token) => {
                                    pinnacle.loop_handle.remove(token);
                                }
                            };

                            // From niri: We queued this frame successfully, so the current client buffers were
                            // latched. We can send frame callbacks now, since a new client commit
                            // will no longer overwrite this frame and will wait for a VBlank.
                            surface.frame_callback_sequence.increment();

                            pinnacle.send_frame_callbacks(
                                output,
                                Some(surface.frame_callback_sequence),
                            );

                            self.update_output_vrr(pinnacle, output);

                            // Return here to not queue the estimated vblank timer on a submitted frame
                            return;
                        }
                        Err(err) => {
                            warn!("Error queueing frame: {err}");
                            true
                        }
                    }
                } else {
                    false
                }
            }
            Err(err) => {
                // Can fail if we switched to a different TTY
                warn!("Render failed for surface: {err}");
                true
            }
        };

        Self::queue_estimated_vblank_timer(surface, pinnacle, output, time_to_next_presentation);

        if failed {
            surface.render_state = if let RenderState::WaitingForEstimatedVblank(token)
            | RenderState::WaitingForEstimatedVblankAndScheduled(
                token,
            ) = surface.render_state
            {
                RenderState::WaitingForEstimatedVblank(token)
            } else {
                RenderState::Idle
            };
        }

        pinnacle.send_frame_callbacks(output, Some(surface.frame_callback_sequence));
    }

    fn queue_estimated_vblank_timer(
        surface: &mut RenderSurface,
        pinnacle: &mut Pinnacle,
        output: &Output,
        mut time_to_next_presentation: Duration,
    ) {
        let span = tracy_client::span!("Udev::queue_estimated_vblank_timer");
        span.emit_text(&output.name());

        match mem::take(&mut surface.render_state) {
            RenderState::Idle => unreachable!(),
            RenderState::Scheduled => (),
            RenderState::WaitingForVblank { .. } => unreachable!(),
            RenderState::WaitingForEstimatedVblank(token)
            | RenderState::WaitingForEstimatedVblankAndScheduled(token) => {
                surface.render_state = RenderState::WaitingForEstimatedVblank(token);
                return;
            }
        }

        // No use setting a zero timer, since we'll send frame callbacks anyway right after the call to
        // render(). This can happen for example with unknown presentation time from DRM.
        if time_to_next_presentation.is_zero() {
            time_to_next_presentation += surface
                .frame_clock
                .refresh_interval()
                // Unknown refresh interval, i.e. winit backend. Would be good to estimate it somehow
                // but it's not that important for this code path.
                .unwrap_or(Duration::from_micros(16_667));
        }

        let timer = Timer::from_duration(time_to_next_presentation);

        let output = output.clone();
        let token = pinnacle
            .loop_handle
            .insert_source(timer, move |_, _, state| {
                state
                    .backend
                    .udev_mut()
                    .on_estimated_vblank_timer(&mut state.pinnacle, &output);
                TimeoutAction::Drop
            })
            .unwrap();

        surface.render_state = RenderState::WaitingForEstimatedVblank(token);
    }

    fn on_estimated_vblank_timer(&mut self, pinnacle: &mut Pinnacle, output: &Output) {
        let span = tracy_client::span!("Udev::on_estimated_vblank_timer");
        span.emit_text(&output.name());

        let Some(surface) = render_surface_for_output(output, &mut self.devices) else {
            return;
        };

        surface.frame_callback_sequence.increment();

        match mem::take(&mut surface.render_state) {
            RenderState::Idle => {
                // FIXME: this is still reachable after sleep
            }
            RenderState::Scheduled => unreachable!(),
            RenderState::WaitingForVblank { .. } => unreachable!(),
            RenderState::WaitingForEstimatedVblank(_) => (),
            RenderState::WaitingForEstimatedVblankAndScheduled(_) => {
                surface.render_state = RenderState::Scheduled;
                return;
            }
        }

        if pinnacle.cursor_state.is_current_cursor_animated() {
            self.schedule_render(output);
        } else {
            pinnacle.send_frame_callbacks(output, Some(surface.frame_callback_sequence));
        }
    }

    fn update_output_vrr(&mut self, pinnacle: &mut Pinnacle, output: &Output) {
        if output.with_state(|state| !state.is_vrr_on_demand) {
            return;
        }

        let vrr = pinnacle.space.elements_for_output(output).any(|win| {
            let Some(demand) = win.with_state(|state| state.vrr_demand) else {
                return false;
            };

            let mut visible = false;
            win.with_surfaces(|surface, states| {
                if surface_primary_scanout_output(surface, states).as_ref() == Some(output) {
                    visible = true;
                }
            });

            visible
                // FIXME: We probably want to check the actual fullscreen state, not the layout mode,
                // but this isn't a *super* huge deal
                && (!demand.fullscreen || win.with_state(|state| state.layout_mode.is_fullscreen()))
        });

        self.set_output_vrr(output, vrr);
    }

    pub(super) fn set_output_vrr(&mut self, output: &Output, vrr: bool) {
        let Some(surface) = render_surface_for_output(output, &mut self.devices) else {
            return;
        };

        if surface.frame_clock.vrr() == vrr {
            return;
        }

        info!(
            "{} vrr on output {}",
            if vrr { "Enabling" } else { "Disabling" },
            output.name()
        );

        if let Err(err) = surface.drm_output.with_compositor(|comp| comp.use_vrr(vrr)) {
            warn!("Failed to set vrr on output {}: {err}", output.name());
        }
        surface.frame_clock.set_vrr(
            surface
                .drm_output
                .with_compositor(|comp| comp.vrr_enabled()),
        );
        output.with_state_mut(|state| state.is_vrr_on = surface.frame_clock.vrr());
    }
}

fn render_surface_for_output<'a>(
    output: &Output,
    devices: &'a mut HashMap<DrmNode, Device>,
) -> Option<&'a mut RenderSurface> {
    let UdevOutputData { device_id, crtc } = output.user_data().get()?;

    devices
        .get_mut(device_id)
        .and_then(|device| device.surfaces.get_mut(crtc))
}

// FIXME: damage is completely wrong lol, totally didn't test that
// Use an OutputDamageTracker or something
fn handle_pending_screencopy<'a>(
    renderer: &mut UdevRenderer<'a>,
    output: &Output,
    surface: &mut RenderSurface,
    render_frame_result: &UdevRenderFrameResult<'a>,
    loop_handle: &LoopHandle<'static, State>,
    cursor_ids: Vec<Id>,
) {
    let span = tracy_client::span!("udev::handle_pending_screencopy");
    span.emit_text(&output.name());

    let screencopies =
        output.with_state_mut(|state| state.screencopies.drain(..).collect::<Vec<_>>());

    for mut screencopy in screencopies {
        assert_eq!(screencopy.output(), output);

        let untransformed_output_size = output.current_mode().expect("output no mode").size;

        let scale = smithay::utils::Scale::from(output.current_scale().fractional_scale());

        if screencopy.with_damage() {
            if render_frame_result.is_empty {
                output.with_state_mut(|state| state.screencopies.push(screencopy));
                continue;
            }

            // Compute damage
            //
            // I have no idea if the damage event is supposed to send rects local to the output or to the
            // region. Sway does the former, Hyprland the latter. Also, no one actually seems to be using the
            // received damage. wf-recorder and wl-mirror have no-op handlers for the damage event.

            let mut damage = match &render_frame_result.primary_element {
                PrimaryPlaneElement::Swapchain(element) => {
                    let swapchain_commit =
                        &mut surface.screencopy_commit_state.primary_plane_swapchain;
                    let damage = element.damage.damage_since(Some(*swapchain_commit));
                    *swapchain_commit = element.damage.current_commit();
                    damage.map(|dmg| {
                        dmg.into_iter()
                            .map(|rect| {
                                rect.to_logical(1, Transform::Normal, &rect.size)
                                    .to_physical(1)
                            })
                            .collect()
                    })
                }
                PrimaryPlaneElement::Element(element) => {
                    // INFO: Is this element guaranteed to be the same size as the
                    // |     output? If not this becomes a
                    // FIXME: offset the damage by the element's location
                    //
                    // also is this even ever reachable?
                    let element_commit = &mut surface.screencopy_commit_state.primary_plane_element;
                    let damage = element.damage_since(scale, Some(*element_commit));
                    *element_commit = element.current_commit();
                    Some(damage)
                }
            }
            .unwrap_or_else(|| {
                // Returning `None` means the previous CommitCounter is too old or damage
                // was reset, so damage the whole output
                DamageSet::from_slice(&[Rectangle::from_size(untransformed_output_size)])
            });

            let cursor_damage = render_frame_result
                .cursor_element
                .map(|cursor| {
                    let damage =
                        cursor.damage_since(scale, Some(surface.screencopy_commit_state.cursor));
                    surface.screencopy_commit_state.cursor = cursor.current_commit();
                    damage
                })
                .unwrap_or_default();

            damage = damage.into_iter().chain(cursor_damage).collect();

            // The primary plane and cursor had no damage but something got rendered,
            // so it must be the cursor moving.
            //
            // We currently have overlay planes disabled, so we don't have to worry about that.
            if damage.is_empty()
                && !render_frame_result.is_empty
                && let Some(cursor_elem) = render_frame_result.cursor_element
            {
                damage = damage
                    .into_iter()
                    .chain([cursor_elem.geometry(scale)])
                    .collect();
            }

            // INFO: Protocol states that `copy_with_damage` should wait until there is
            // |     damage to be copied.
            // |.
            // |     Now, for region screencopies this currently submits the frame if there is
            // |     *any* damage on the output, not just in the region. I've found that
            // |     wf-recorder blocks until the last frame is submitted, and if I don't
            // |     send a submission because its region isn't damaged it will hang.
            // |     I'm fairly certain Sway is doing a similar thing.
            if damage.is_empty() {
                output.with_state_mut(|state| state.screencopies.push(screencopy));
                continue;
            }

            screencopy.damage(&damage);
        }

        let sync_point = if let Ok(mut dmabuf) = dmabuf::get_dmabuf(screencopy.buffer()).cloned() {
            trace!("Dmabuf screencopy");

            let format_correct =
                Some(dmabuf.format().code) == shm_format_to_fourcc(wl_shm::Format::Argb8888);
            let width_correct = dmabuf.width() == screencopy.physical_region().size.w as u32;
            let height_correct = dmabuf.height() == screencopy.physical_region().size.h as u32;

            if !(format_correct && width_correct && height_correct) {
                continue;
            }

            (|| -> anyhow::Result<Option<SyncPoint>> {
                if screencopy.physical_region() == Rectangle::from_size(untransformed_output_size) {
                    // Optimization to not have to do an extra blit;
                    // just blit the whole output
                    let mut framebuffer = renderer.bind(&mut dmabuf)?;

                    Ok(Some(render_frame_result.blit_frame_result(
                        screencopy.physical_region().size,
                        Transform::Normal,
                        output.current_scale().fractional_scale(),
                        renderer,
                        &mut framebuffer,
                        [screencopy.physical_region()],
                        if !screencopy.overlay_cursor() {
                            cursor_ids.clone()
                        } else {
                            Vec::new()
                        },
                    )?))
                } else {
                    // `RenderFrameResult::blit_frame_result` doesn't expose a way to
                    // blit from a source rectangle, so blit into another buffer
                    // then blit from that into the dmabuf.

                    let output_buffer_size = untransformed_output_size
                        .to_logical(1)
                        .to_buffer(1, Transform::Normal);

                    let mut offscreen: GlesRenderbuffer = renderer.create_buffer(
                        smithay::backend::allocator::Fourcc::Abgr8888,
                        output_buffer_size,
                    )?;

                    let mut offscreen_fb = renderer.bind(&mut offscreen)?;

                    // TODO: Figure out if this sync point needs waiting
                    let _ = render_frame_result.blit_frame_result(
                        untransformed_output_size,
                        Transform::Normal,
                        output.current_scale().fractional_scale(),
                        renderer,
                        &mut offscreen_fb,
                        [Rectangle::from_size(untransformed_output_size)],
                        if !screencopy.overlay_cursor() {
                            cursor_ids.clone()
                        } else {
                            Vec::new()
                        },
                    )?;

                    let mut dmabuf_fb = renderer.bind(&mut dmabuf)?;

                    let sync_point = renderer.blit(
                        &offscreen_fb,
                        &mut dmabuf_fb,
                        screencopy.physical_region(),
                        Rectangle::from_size(screencopy.physical_region().size),
                        TextureFilter::Linear,
                    )?;

                    Ok(Some(sync_point))
                }
            })()
        } else if !matches!(
            renderer::buffer_type(screencopy.buffer()),
            Some(BufferType::Shm)
        ) {
            Err(anyhow!("not a shm buffer"))
        } else {
            trace!("Shm screencopy");

            let res = smithay::wayland::shm::with_buffer_contents_mut(
                &screencopy.buffer().clone(),
                |shm_ptr, shm_len, buffer_data| {
                    // yoinked from Niri (thanks yall)
                    ensure!(
                        // The buffer prefers pixels in little endian ...
                        buffer_data.format == wl_shm::Format::Argb8888
                            && buffer_data.stride == screencopy.physical_region().size.w * 4
                            && buffer_data.height == screencopy.physical_region().size.h
                            && shm_len as i32 == buffer_data.stride * buffer_data.height,
                        "invalid buffer format or size"
                    );

                    let src_buffer_rect = screencopy.physical_region().to_logical(1).to_buffer(
                        1,
                        Transform::Normal,
                        &screencopy.physical_region().size.to_logical(1),
                    );

                    let output_buffer_size = untransformed_output_size
                        .to_logical(1)
                        .to_buffer(1, Transform::Normal);

                    let mut offscreen: GlesRenderbuffer = renderer.create_buffer(
                        smithay::backend::allocator::Fourcc::Abgr8888,
                        output_buffer_size,
                    )?;

                    let mut framebuffer = renderer.bind(&mut offscreen)?;

                    // Blit the entire output to `offscreen`.
                    // Only the needed region will be copied below
                    let sync_point = render_frame_result.blit_frame_result(
                        untransformed_output_size,
                        Transform::Normal,
                        output.current_scale().fractional_scale(),
                        renderer,
                        &mut framebuffer,
                        [Rectangle::from_size(untransformed_output_size)],
                        if !screencopy.overlay_cursor() {
                            cursor_ids.clone()
                        } else {
                            Vec::new()
                        },
                    )?;

                    // Can someone explain to me why it feels like some things are
                    // arbitrarily `Physical` or `Buffer`
                    let mapping = renderer.copy_framebuffer(
                        &framebuffer,
                        src_buffer_rect,
                        smithay::backend::allocator::Fourcc::Argb8888,
                    )?;

                    let bytes = renderer.map_texture(&mapping)?;

                    ensure!(bytes.len() == shm_len, "mapped buffer has wrong length");

                    // SAFETY: TODO: safety docs
                    unsafe {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), shm_ptr, shm_len);
                    }

                    Ok(Some(sync_point))
                },
            );

            let Ok(res) = res else {
                unreachable!();
            };

            res
        };

        match sync_point {
            Ok(Some(sync_point)) if !sync_point.is_reached() => {
                let Some(sync_fd) = sync_point.export() else {
                    screencopy.submit(false);
                    continue;
                };
                let mut screencopy = Some(screencopy);
                let source = Generic::new(sync_fd, Interest::READ, calloop::Mode::OneShot);
                let res = loop_handle.insert_source(source, move |_, _, _| {
                    let Some(screencopy) = screencopy.take() else {
                        unreachable!("This source is removed after one run");
                    };
                    screencopy.submit(false);
                    trace!("Submitted screencopy");
                    Ok(PostAction::Remove)
                });
                if res.is_err() {
                    error!("Failed to schedule screencopy submission");
                }
            }
            Ok(_) => screencopy.submit(false),
            Err(err) => error!("Failed to submit screencopy: {err}"),
        }
    }
}
