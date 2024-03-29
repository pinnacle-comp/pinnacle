// SPDX-License-Identifier: GPL-3.0-or-later

mod drm_util;

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use pinnacle_api_defs::pinnacle::signal::v0alpha1::OutputConnectResponse;
use smithay::{
    backend::{
        allocator::{
            dmabuf::{AnyError, Dmabuf, DmabufAllocator},
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            vulkan::{ImageUsageFlags, VulkanAllocator},
            Allocator, Fourcc,
        },
        drm::{
            compositor::{DrmCompositor, PrimaryPlaneElement},
            CreateDrmNodeError, DrmDevice, DrmDeviceFd, DrmError, DrmEvent, DrmEventMetadata,
            DrmNode, NodeType,
        },
        egl::{self, EGLDevice, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage,
            element::{texture::TextureBuffer, RenderElement, RenderElementStates},
            gles::GlesRenderer,
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer, MultiTexture},
            Bind, ImportDma, ImportEgl, ImportMemWl, Renderer, TextureFilter,
        },
        session::{
            self,
            libseat::{self, LibSeatSession},
            Session,
        },
        udev::{self, UdevBackend, UdevEvent},
        vulkan::{self, version::Version, PhysicalDevice},
        SwapBuffersError,
    },
    desktop::{
        layer_map_for_output,
        utils::{send_frames_surface_tree, OutputPresentationFeedback},
        Space,
    },
    input::pointer::CursorImageStatus,
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        ash::vk::ExtPhysicalDeviceDrmFn,
        calloop::{EventLoop, Idle, LoopHandle, RegistrationToken},
        drm::control::{connector, crtc, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_protocols::wp::{
            linux_dmabuf::zv1::server::zwp_linux_dmabuf_feedback_v1,
            presentation_time::server::wp_presentation_feedback,
        },
        wayland_server::{
            backend::GlobalId, protocol::wl_surface::WlSurface, Display, DisplayHandle,
        },
    },
    utils::{Clock, DeviceFd, Logical, Monotonic, Point, Transform},
    wayland::dmabuf::{DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};
use tracing::{error, warn};

use crate::{
    backend::Backend,
    config::ConnectorSavedState,
    output::OutputName,
    render::{pointer::PointerElement, take_presentation_feedback},
    state::{State, SurfaceDmabufFeedback, WithState},
    window::WindowElement,
};

use self::drm_util::EdidInfo;

use super::BackendData;

const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];
const SUPPORTED_FORMATS_8BIT_ONLY: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];

/// A [`MultiRenderer`] that uses the [`GbmGlesBackend`].
type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

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
    display_handle: DisplayHandle,
    pub(super) dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub(super) primary_gpu: DrmNode,
    allocator: Option<Box<dyn Allocator<Buffer = Dmabuf, Error = AnyError>>>,
    pub(super) gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    backends: HashMap<DrmNode, UdevBackendData>,
    pointer_images: Vec<(xcursor::parser::Image, TextureBuffer<MultiTexture>)>,
    pointer_element: PointerElement<MultiTexture>,
    pointer_image: crate::cursor::Cursor,

    pub(super) upscale_filter: TextureFilter,
    pub(super) downscale_filter: TextureFilter,
}

impl Backend {
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
    /// Schedule a new render that will cause the compositor to redraw everything.
    pub fn schedule_render(&mut self, loop_handle: &LoopHandle<State>, output: &Output) {
        let Some(surface) = render_surface_for_output(output, &mut self.backends) else {
            return;
        };

        match &surface.render_state {
            RenderState::Idle => {
                let output = output.clone();
                let token = loop_handle.insert_idle(move |state| {
                    state.render_surface(&output);
                });

                surface.render_state = RenderState::Scheduled(token);
            }
            RenderState::Scheduled(_) => (),
            RenderState::WaitingForVblank { dirty: _ } => {
                surface.render_state = RenderState::WaitingForVblank { dirty: true }
            }
        }
    }
}

impl State {
    /// Switch the tty.
    ///
    /// This will first clear the overlay plane to prevent any lingering artifacts,
    /// then switch the vt.
    ///
    /// Does nothing when called on the winit backend.
    pub fn switch_vt(&mut self, vt: i32) {
        if let Backend::Udev(udev) = &mut self.backend {
            for backend in udev.backends.values_mut() {
                for surface in backend.surfaces.values_mut() {
                    // Clear the overlay planes on tty switch.
                    //
                    // On my machine, switching a tty would leave the topmost window on the
                    // screen. Smithay will render the topmost window on the overlay plane,
                    // so we clear it here.
                    let planes = surface.compositor.surface().planes().clone();
                    tracing::debug!("Clearing overlay planes");
                    for overlay_plane in planes.overlay {
                        if let Err(err) = surface
                            .compositor
                            .surface()
                            .clear_plane(overlay_plane.handle)
                        {
                            warn!("Failed to clear overlay planes: {err}");
                        }
                    }
                }
            }

            // Wait for the clear to commit before switching
            self.schedule(
                |state| {
                    let udev = state.backend.udev();
                    !udev
                        .backends
                        .values()
                        .flat_map(|backend| backend.surfaces.values())
                        .map(|surface| surface.compositor.surface())
                        .any(|drm_surf| drm_surf.commit_pending())
                },
                move |state| {
                    let udev = state.backend.udev_mut();
                    if let Err(err) = udev.session.change_vt(vt) {
                        error!("Failed to switch to vt {vt}: {err}");
                    }
                },
            );
        }
    }

    /// Resize the output with the given mode.
    ///
    /// TODO: This is in udev.rs but is also used in winit.rs.
    /// |     I've got no clue how to make things public without making a mess.
    pub fn resize_output(&mut self, output: &Output, mode: smithay::output::Mode) {
        if let Backend::Udev(udev) = &mut self.backend {
            let drm_mode = udev.backends.iter().find_map(|(_, backend)| {
                backend
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
                            .find(|m| smithay::output::Mode::from(**m) == mode)
                    })
                    .copied()
            });

            if let Some(drm_mode) = drm_mode {
                if let Some(render_surface) = render_surface_for_output(output, &mut udev.backends)
                {
                    match render_surface.compositor.use_mode(drm_mode) {
                        Ok(()) => {
                            output.change_current_state(Some(mode), None, None, None);
                            layer_map_for_output(output).arrange();
                        }
                        Err(err) => error!("Failed to resize output: {err}"),
                    }
                }
            }
        } else {
            output.change_current_state(Some(mode), None, None, None);
            layer_map_for_output(output).arrange();
        }

        self.schedule_render(output);
        self.request_layout(output);
    }
}

impl BackendData for Udev {
    fn seat_name(&self) -> String {
        self.session.seat()
    }

    fn reset_buffers(&mut self, output: &Output) {
        if let Some(id) = output.user_data().get::<UdevOutputData>() {
            if let Some(gpu) = self.backends.get_mut(&id.device_id) {
                if let Some(surface) = gpu.surfaces.get_mut(&id.crtc) {
                    surface.compositor.reset_buffers();
                }
            }
        }
    }

    fn early_import(&mut self, surface: &WlSurface) {
        if let Err(err) = self.gpu_manager.early_import(self.primary_gpu, surface) {
            warn!("early buffer import failed: {}", err);
        }
    }
}

pub fn setup_udev(
    no_config: bool,
    config_dir: Option<PathBuf>,
) -> anyhow::Result<(State, EventLoop<'static, State>)> {
    let event_loop = EventLoop::try_new()?;
    let display = Display::new()?;

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
    tracing::info!("Using {} as primary gpu.", primary_gpu);

    let gpu_manager = GpuManager::new(GbmGlesBackend::default())?;

    let data = Udev {
        display_handle: display.handle(),
        dmabuf_state: None,
        session,
        primary_gpu,
        gpu_manager,
        allocator: None,
        backends: HashMap::new(),
        pointer_image: crate::cursor::Cursor::load(),
        pointer_images: Vec::new(),
        pointer_element: PointerElement::default(),

        upscale_filter: TextureFilter::Linear,
        downscale_filter: TextureFilter::Linear,
    };

    let display_handle = display.handle();

    let mut state = State::init(
        Backend::Udev(data),
        display,
        event_loop.get_signal(),
        event_loop.handle(),
        no_config,
        config_dir,
    )?;

    // Initialize the udev backend
    let udev_backend = UdevBackend::new(state.seat.name())?;

    // Create DrmNodes from already connected GPUs
    for (device_id, path) in udev_backend.device_list() {
        if let Err(err) = DrmNode::from_dev_id(device_id)
            .map_err(DeviceAddError::DrmNode)
            .and_then(|node| state.device_added(node, path))
        {
            error!("Skipping device {device_id}: {err}");
        }
    }

    let udev = state.backend.udev_mut();

    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, state| match event {
            // GPU connected
            UdevEvent::Added { device_id, path } => {
                if let Err(err) = DrmNode::from_dev_id(device_id)
                    .map_err(DeviceAddError::DrmNode)
                    .and_then(|node| state.device_added(node, &path))
                {
                    error!("Skipping device {device_id}: {err}");
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.device_changed(node)
                }
            }
            // GPU disconnected
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    state.device_removed(node)
                }
            }
        })
        .expect("failed to insert udev_backend into event loop");

    // Initialize libinput backend
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        udev.session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(state.seat.name())
        .expect("failed to assign seat to libinput");
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // Bind all our objects that get driven by the event loop

    let insert_ret = event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, state| {
            state.apply_libinput_settings(&event);
            state.process_input_event(event);
        });

    if let Err(err) = insert_ret {
        anyhow::bail!("Failed to insert libinput_backend into event loop: {err}");
    }

    event_loop
        .handle()
        .insert_source(notifier, move |event, _, state| {
            let udev = state.backend.udev_mut();

            match event {
                session::Event::PauseSession => {
                    libinput_context.suspend();
                    tracing::info!("pausing session");

                    for backend in udev.backends.values_mut() {
                        backend.drm.pause();
                    }
                }
                session::Event::ActivateSession => {
                    tracing::info!("resuming session");

                    if let Err(err) = libinput_context.resume() {
                        error!("Failed to resume libinput context: {:?}", err);
                    }
                    for backend in udev.backends.values_mut() {
                        // TODO: this is false because i'm too lazy to remove the code directly
                        // |     below it
                        backend.drm.activate(false).expect("failed to activate drm");
                        for surface in backend.surfaces.values_mut() {
                            if let Err(err) = surface.compositor.surface().reset_state() {
                                warn!("Failed to reset drm surface state: {}", err);
                            }
                            // reset the buffers after resume to trigger a full redraw
                            // this is important after a vt switch as the primary plane
                            // has no content and damage tracking may prevent a redraw
                            // otherwise
                            surface.compositor.reset_buffers();
                        }
                    }

                    let connectors = udev
                        .backends
                        .iter()
                        .map(|(node, backend)| {
                            let connectors = backend
                                .drm_scanner
                                .crtcs()
                                .map(|(info, crtc)| (info.clone(), crtc))
                                .collect::<Vec<_>>();
                            (*node, connectors)
                        })
                        .collect::<Vec<_>>();

                    for (node, connectors) in connectors {
                        for (connector, crtc) in connectors {
                            state.connector_disconnected(node, connector.clone(), crtc);
                            state.connector_connected(node, connector, crtc);
                        }
                    }
                    for output in state.space.outputs().cloned().collect::<Vec<_>>() {
                        state.schedule_render(&output);
                    }
                }
            }
        })
        .expect("failed to insert libinput notifier into event loop");

    state.shm_state.update_formats(
        udev.gpu_manager
            .single_renderer(&primary_gpu)?
            .shm_formats(),
    );

    // Create the Vulkan allocator
    if let Ok(instance) = vulkan::Instance::new(Version::VERSION_1_2, None) {
        if let Some(physical_device) =
            PhysicalDevice::enumerate(&instance)
                .ok()
                .and_then(|devices| {
                    devices
                        .filter(|phd| phd.has_device_extension(ExtPhysicalDeviceDrmFn::name()))
                        .find(|phd| {
                            phd.primary_node()
                                .is_ok_and(|node| node == Some(primary_gpu))
                                || phd
                                    .render_node()
                                    .is_ok_and(|node| node == Some(primary_gpu))
                        })
                })
        {
            match VulkanAllocator::new(
                &physical_device,
                ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
            ) {
                Ok(allocator) => {
                    udev.allocator = Some(Box::new(DmabufAllocator(allocator))
                        as Box<dyn Allocator<Buffer = Dmabuf, Error = AnyError>>);
                }
                Err(err) => {
                    warn!("Failed to create vulkan allocator: {}", err);
                }
            }
        }
    }

    if udev.allocator.is_none() {
        tracing::info!("No vulkan allocator found, using GBM.");
        let gbm = udev
            .backends
            .get(&primary_gpu)
            // If the primary_gpu failed to initialize, we likely have a kmsro device
            .or_else(|| udev.backends.values().next())
            // Don't fail, if there is no allocator. There is a chance, that this a single gpu system and we don't need one.
            .map(|backend| backend.gbm.clone());
        udev.allocator = gbm.map(|gbm| {
            Box::new(DmabufAllocator(GbmAllocator::new(
                gbm,
                GbmBufferFlags::RENDERING,
            ))) as Box<_>
        });
    }

    let mut renderer = udev.gpu_manager.single_renderer(&primary_gpu)?;

    tracing::info!(
        ?primary_gpu,
        "Trying to initialize EGL Hardware Acceleration",
    );

    match renderer.bind_wl_display(&display_handle) {
        Ok(_) => tracing::info!("EGL hardware-acceleration enabled"),
        Err(err) => error!(?err, "Failed to initialize EGL hardware-acceleration"),
    }

    // init dmabuf support with format list from our primary gpu
    let dmabuf_formats = renderer.dmabuf_formats().collect::<Vec<_>>();
    let default_feedback = DmabufFeedbackBuilder::new(primary_gpu.dev_id(), dmabuf_formats)
        .build()
        .expect("failed to create dmabuf feedback");
    let mut dmabuf_state = DmabufState::new();
    let global = dmabuf_state
        .create_global_with_default_feedback::<State>(&display_handle, &default_feedback);
    udev.dmabuf_state = Some((dmabuf_state, global));

    let gpu_manager = &mut udev.gpu_manager;
    udev.backends.values_mut().for_each(|backend_data| {
        // Update the per drm surface dmabuf feedback
        backend_data.surfaces.values_mut().for_each(|surface_data| {
            surface_data.dmabuf_feedback = surface_data.dmabuf_feedback.take().or_else(|| {
                get_surface_dmabuf_feedback(
                    primary_gpu,
                    surface_data.render_node,
                    gpu_manager,
                    &surface_data.compositor,
                )
            });
        });
    });

    if let Err(err) = state.xwayland.start(
        state.loop_handle.clone(),
        None,
        std::iter::empty::<(OsString, OsString)>(),
        true,
        |_| {},
    ) {
        error!("Failed to start XWayland: {err}");
    }

    Ok((state, event_loop))
}

// TODO: document desperately
struct UdevBackendData {
    surfaces: HashMap<crtc::Handle, RenderSurface>,
    gbm: GbmDevice<DrmDeviceFd>,
    drm: DrmDevice,
    drm_scanner: DrmScanner,
    render_node: DrmNode,
    registration_token: RegistrationToken,
}

#[derive(Debug, thiserror::Error)]
enum DeviceAddError {
    #[error("Failed to open device using libseat: {0}")]
    DeviceOpen(libseat::Error),
    #[error("Failed to initialize drm device: {0}")]
    DrmDevice(DrmError),
    #[error("Failed to initialize gbm device: {0}")]
    GbmDevice(std::io::Error),
    #[error("Failed to access drm node: {0}")]
    DrmNode(CreateDrmNodeError),
    #[error("Failed to add device to GpuManager: {0}")]
    AddNode(egl::Error),
}

fn get_surface_dmabuf_feedback(
    primary_gpu: DrmNode,
    render_node: DrmNode,
    gpu_manager: &mut GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    composition: &GbmDrmCompositor,
) -> Option<DrmSurfaceDmabufFeedback> {
    let primary_formats = gpu_manager
        .single_renderer(&primary_gpu)
        .ok()?
        .dmabuf_formats()
        .collect::<HashSet<_>>();

    let render_formats = gpu_manager
        .single_renderer(&render_node)
        .ok()?
        .dmabuf_formats()
        .collect::<HashSet<_>>();

    let all_render_formats = primary_formats
        .iter()
        .chain(render_formats.iter())
        .copied()
        .collect::<HashSet<_>>();

    let surface = composition.surface();
    let planes = surface.planes().clone();
    // We limit the scan-out trache to formats we can also render from
    // so that there is always a fallback render path available in case
    // the supplied buffer can not be scanned out directly
    let planes_formats = planes
        .primary
        .formats
        .into_iter()
        .chain(planes.overlay.into_iter().flat_map(|p| p.formats))
        .collect::<HashSet<_>>()
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

    Some(DrmSurfaceDmabufFeedback {
        render_feedback,
        scanout_feedback,
    })
}

struct DrmSurfaceDmabufFeedback {
    render_feedback: DmabufFeedback,
    scanout_feedback: DmabufFeedback,
}

/// The state of a [`RenderSurface`].
#[derive(Debug)]
enum RenderState {
    /// No render is scheduled.
    Idle,
    // TODO: remove the token on tty switch or output unplug
    /// A render has been queued.
    Scheduled(
        /// The idle token from a render being scheduled.
        /// This is used to cancel renders if, for example,
        /// the output being rendered is removed.
        #[allow(dead_code)] // TODO:
        Idle<'static>,
    ),
    /// A frame was rendered and scheduled and we are waiting for vblank.
    WaitingForVblank {
        /// A render was scheduled while waiting for vblank.
        /// In this case, another render will be scheduled once vblank happens.
        dirty: bool,
    },
}

/// Render surface for an output.
struct RenderSurface {
    /// The output global id.
    global: Option<GlobalId>,
    /// A display handle used to remove the global on drop.
    display_handle: DisplayHandle,
    /// The node from `connector_connected`.
    device_id: DrmNode,
    /// The node rendering to the screen? idk
    ///
    /// If this is equal to the primary gpu node then it does the rendering operations.
    /// If it's not it is the node the composited buffer ends up on.
    render_node: DrmNode,
    /// The thing rendering elements and queueing frames.
    compositor: GbmDrmCompositor,
    dmabuf_feedback: Option<DrmSurfaceDmabufFeedback>,
    render_state: RenderState,
}

impl Drop for RenderSurface {
    // Stop advertising this output to clients on drop.
    fn drop(&mut self) {
        if let Some(global) = self.global.take() {
            self.display_handle.remove_global::<State>(global);
        }
    }
}

type GbmDrmCompositor = DrmCompositor<
    GbmAllocator<DrmDeviceFd>,
    GbmDevice<DrmDeviceFd>,
    Option<OutputPresentationFeedback>,
    DrmDeviceFd,
>;

/// The result of a frame render from `render_frame`.
struct SurfaceCompositorRenderResult {
    rendered: bool,
    states: RenderElementStates,
}

/// Render a frame with the given elements.
///
/// This frame needs to be queued for scanout afterwards.
fn render_frame<R, E>(
    compositor: &mut GbmDrmCompositor,
    renderer: &mut R,
    elements: &[E],
    clear_color: [f32; 4],
) -> Result<SurfaceCompositorRenderResult, SwapBuffersError>
where
    R: Renderer + Bind<Dmabuf>,
    <R as Renderer>::TextureId: 'static,
    <R as Renderer>::Error: Into<SwapBuffersError>,
    E: RenderElement<R>,
{
    use smithay::backend::drm::compositor::RenderFrameError;

    compositor
        .render_frame(renderer, elements, clear_color)
        .map(|render_frame_result| {
            if let PrimaryPlaneElement::Swapchain(element) = render_frame_result.primary_element {
                element.sync.wait();
            }
            SurfaceCompositorRenderResult {
                rendered: !render_frame_result.is_empty,
                states: render_frame_result.states,
            }
        })
        .map_err(|err| match err {
            RenderFrameError::PrepareFrame(err) => err.into(),
            RenderFrameError::RenderFrame(damage::Error::Rendering(err)) => err.into(),
            _ => unreachable!(),
        })
}

impl State {
    /// A GPU was plugged in.
    fn device_added(&mut self, node: DrmNode, path: &Path) -> Result<(), DeviceAddError> {
        let udev = self.backend.udev_mut();

        // Try to open the device
        let fd = udev
            .session
            .open(
                path,
                OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
            )
            .map_err(DeviceAddError::DeviceOpen)?;

        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (drm, notifier) =
            DrmDevice::new(fd.clone(), true).map_err(DeviceAddError::DrmDevice)?;
        let gbm = GbmDevice::new(fd).map_err(DeviceAddError::GbmDevice)?;

        let registration_token = self
            .loop_handle
            .insert_source(notifier, move |event, metadata, state| match event {
                DrmEvent::VBlank(crtc) => {
                    state.on_vblank(node, crtc, metadata);
                }
                DrmEvent::Error(error) => {
                    error!("{:?}", error);
                }
            })
            .expect("failed to insert drm notifier into event loop");

        // SAFETY: no clue lol just copied this from anvil
        let render_node = EGLDevice::device_for_display(&unsafe {
            EGLDisplay::new(gbm.clone()).expect("failed to create EGLDisplay")
        })
        .ok()
        .and_then(|x| x.try_get_render_node().ok().flatten())
        .unwrap_or(node);

        udev.gpu_manager
            .as_mut()
            .add_node(render_node, gbm.clone())
            .map_err(DeviceAddError::AddNode)?;

        udev.backends.insert(
            node,
            UdevBackendData {
                registration_token,
                gbm,
                drm,
                drm_scanner: DrmScanner::new(),
                render_node,
                surfaces: HashMap::new(),
            },
        );

        self.device_changed(node);

        Ok(())
    }

    /// A display was plugged in.
    fn connector_connected(
        &mut self,
        node: DrmNode,
        connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        let udev = self.backend.udev_mut();

        let device = if let Some(device) = udev.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        let mut renderer = udev
            .gpu_manager
            .single_renderer(&device.render_node)
            .expect("failed to get primary gpu MultiRenderer");
        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .clone();

        tracing::info!(
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
        let wl_mode = smithay::output::Mode::from(drm_mode);

        let modes = connector
            .modes()
            .iter()
            .map(|mode| smithay::output::Mode::from(*mode));

        let surface = match device
            .drm
            .create_surface(crtc, drm_mode, &[connector.handle()])
        {
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

        let (make, model) = EdidInfo::try_from_connector(&device.drm, connector.handle())
            .map(|info| (info.manufacturer, info.model))
            .unwrap_or_else(|err| {
                warn!("Failed to parse EDID info: {err}");
                ("Unknown".into(), "Unknown".into())
            });

        let (phys_w, phys_h) = connector.size().unwrap_or((0, 0));

        if self.space.outputs().any(|op| {
            op.user_data()
                .get::<UdevOutputData>()
                .is_some_and(|op_id| op_id.crtc == crtc)
        }) {
            return;
        }

        let output = Output::new(
            output_name,
            PhysicalProperties {
                size: (phys_w as i32, phys_h as i32).into(),
                subpixel: Subpixel::from(connector.subpixel()),
                make,
                model,
            },
        );
        let global = output.create_global::<State>(&udev.display_handle);

        for mode in modes {
            output.add_mode(mode);
        }

        self.output_focus_stack.set_focus(output.clone());

        let x = self.space.outputs().fold(0, |acc, o| {
            let Some(geo) = self.space.output_geometry(o) else {
                unreachable!()
            };
            acc + geo.size.w
        });
        let position = (x, 0).into();

        output.set_preferred(wl_mode);
        output.change_current_state(Some(wl_mode), None, None, Some(position));
        self.space.map_output(&output, position);

        output.user_data().insert_if_missing(|| UdevOutputData {
            crtc,
            device_id: node,
        });

        let allocator = GbmAllocator::new(
            device.gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );

        // I like how this is still in here
        let color_formats = if std::env::var("ANVIL_DISABLE_10BIT").is_ok() {
            SUPPORTED_FORMATS_8BIT_ONLY
        } else {
            SUPPORTED_FORMATS
        };

        let compositor = {
            let mut planes = surface.planes().clone();

            // INFO: We are disabling overlay planes because it seems that any elements on
            // |     overlay planes don't get up/downscaled according to the set filter;
            // |     it always defaults to linear.
            planes.overlay.clear();

            match DrmCompositor::new(
                &output,
                surface,
                Some(planes),
                allocator,
                device.gbm.clone(),
                color_formats,
                render_formats,
                device.drm.cursor_size(),
                Some(device.gbm.clone()),
            ) {
                Ok(compositor) => compositor,
                Err(err) => {
                    warn!("Failed to create drm compositor: {}", err);
                    return;
                }
            }
        };

        let dmabuf_feedback = get_surface_dmabuf_feedback(
            udev.primary_gpu,
            device.render_node,
            &mut udev.gpu_manager,
            &compositor,
        );

        let surface = RenderSurface {
            display_handle: udev.display_handle.clone(),
            device_id: node,
            render_node: device.render_node,
            global: Some(global),
            compositor,
            dmabuf_feedback,
            render_state: RenderState::Idle,
        };

        device.surfaces.insert(crtc, surface);

        // If there is saved connector state, the connector was previously plugged in.
        // In this case, restore its tags and location.
        // TODO: instead of checking the connector, check the monitor's edid info instead
        if let Some(saved_state) = self
            .config
            .connector_saved_states
            .get(&OutputName(output.name()))
        {
            let ConnectorSavedState { loc, tags } = saved_state;

            output.change_current_state(None, None, None, Some(*loc));
            self.space.map_output(&output, *loc);

            output.with_state_mut(|state| state.tags = tags.clone());
        } else {
            self.signal_state.output_connect.signal(|buffer| {
                buffer.push_back(OutputConnectResponse {
                    output_name: Some(output.name()),
                })
            });
        }
    }

    /// A display was unplugged.
    fn connector_disconnected(
        &mut self,
        node: DrmNode,
        _connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        tracing::debug!(?crtc, "connector_disconnected");

        let udev = self.backend.udev_mut();

        let device = if let Some(device) = udev.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        device.surfaces.remove(&crtc);

        let output = self
            .space
            .outputs()
            .find(|o| {
                o.user_data()
                    .get::<UdevOutputData>()
                    .map(|id| id.device_id == node && id.crtc == crtc)
                    .unwrap_or(false)
            })
            .cloned();

        if let Some(output) = output {
            // Save this output's state. It will be restored if the monitor gets replugged.
            self.config.connector_saved_states.insert(
                OutputName(output.name()),
                ConnectorSavedState {
                    loc: output.current_location(),
                    tags: output.with_state(|state| state.tags.clone()),
                },
            );
            self.space.unmap_output(&output);
        }
    }

    fn device_changed(&mut self, node: DrmNode) {
        let udev = self.backend.udev_mut();

        let device = if let Some(device) = udev.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        for event in device.drm_scanner.scan_connectors(&device.drm) {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_connected(node, connector, crtc);
                }
                DrmScanEvent::Disconnected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_disconnected(node, connector, crtc);
                }
                _ => {}
            }
        }
    }

    /// A GPU was unplugged.
    fn device_removed(&mut self, node: DrmNode) {
        let crtcs = {
            let udev = self.backend.udev();

            let Some(device) = udev.backends.get(&node) else {
                return;
            };

            device
                .drm_scanner
                .crtcs()
                .map(|(info, crtc)| (info.clone(), crtc))
                .collect::<Vec<_>>()
        };

        for (connector, crtc) in crtcs {
            self.connector_disconnected(node, connector, crtc);
        }

        tracing::debug!("Surfaces dropped");

        let udev = self.backend.udev_mut();

        // drop the backends on this side
        if let Some(backend_data) = udev.backends.remove(&node) {
            udev.gpu_manager
                .as_mut()
                .remove_node(&backend_data.render_node);

            self.loop_handle.remove(backend_data.registration_token);

            tracing::debug!("Dropping device");
        }
    }

    /// Mark [`OutputPresentationFeedback`]s as presented and schedule a new render on idle.
    fn on_vblank(
        &mut self,
        dev_id: DrmNode,
        crtc: crtc::Handle,
        metadata: &mut Option<DrmEventMetadata>,
    ) {
        let udev = self.backend.udev_mut();

        let Some(surface) = udev
            .backends
            .get_mut(&dev_id)
            .and_then(|device| device.surfaces.get_mut(&crtc))
        else {
            return;
        };

        let output = if let Some(output) = self.space.outputs().find(|o| {
            let udev_op_data = o.user_data().get::<UdevOutputData>();
            udev_op_data
                .is_some_and(|data| data.device_id == surface.device_id && data.crtc == crtc)
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        match surface
            .compositor
            .frame_submitted()
            .map_err(SwapBuffersError::from)
        {
            Ok(user_data) => {
                if let Some(mut feedback) = user_data.flatten() {
                    let tp = metadata.as_ref().and_then(|metadata| match metadata.time {
                        smithay::backend::drm::DrmEventTime::Monotonic(tp) => Some(tp),
                        smithay::backend::drm::DrmEventTime::Realtime(_) => None,
                    });
                    let seq = metadata
                        .as_ref()
                        .map(|metadata| metadata.sequence)
                        .unwrap_or(0);

                    let (clock, flags) = if let Some(tp) = tp {
                        (
                            tp.into(),
                            wp_presentation_feedback::Kind::Vsync
                                | wp_presentation_feedback::Kind::HwClock
                                | wp_presentation_feedback::Kind::HwCompletion,
                        )
                    } else {
                        (self.clock.now(), wp_presentation_feedback::Kind::Vsync)
                    };

                    feedback.presented(
                        clock,
                        output
                            .current_mode()
                            .map(|mode| Duration::from_secs_f64(1000f64 / mode.refresh as f64))
                            .unwrap_or_default(),
                        seq as u64,
                        flags,
                    );
                }
            }
            Err(err) => {
                warn!("Error during rendering: {:?}", err);
                if let SwapBuffersError::ContextLost(err) = err {
                    panic!("Rendering loop lost: {}", err)
                }
            }
        };

        let RenderState::WaitingForVblank { dirty } = surface.render_state else {
            unreachable!();
        };

        surface.render_state = RenderState::Idle;

        if dirty {
            self.schedule_render(&output);
        } else {
            for window in self.windows.iter() {
                window.send_frame(&output, self.clock.now(), Some(Duration::ZERO), |_, _| {
                    Some(output.clone())
                });
            }
        }
    }

    /// Render to the [`RenderSurface`] associated with the given `output`.
    #[tracing::instrument(level = "debug", skip(self), fields(output = output.name()))]
    fn render_surface(&mut self, output: &Output) {
        let udev = self.backend.udev_mut();

        let Some(surface) = render_surface_for_output(output, &mut udev.backends) else {
            return;
        };

        assert!(matches!(surface.render_state, RenderState::Scheduled(_)));

        // TODO get scale from the rendersurface when supporting HiDPI
        let frame = udev.pointer_image.get_image(
            1,
            // output.current_scale().integer_scale() as u32,
            self.clock.now().into(),
        );

        let render_node = surface.render_node;
        let primary_gpu = udev.primary_gpu;
        let mut renderer = if primary_gpu == render_node {
            udev.gpu_manager.single_renderer(&render_node)
        } else {
            let format = surface.compositor.format();
            udev.gpu_manager
                .renderer(&primary_gpu, &render_node, format)
        }
        .expect("failed to create MultiRenderer");

        let _ = renderer.upscale_filter(udev.upscale_filter);
        let _ = renderer.downscale_filter(udev.downscale_filter);

        let pointer_images = &mut udev.pointer_images;
        let pointer_image = pointer_images
            .iter()
            .find_map(
                |(image, texture)| {
                    if image == &frame {
                        Some(texture.clone())
                    } else {
                        None
                    }
                },
            )
            .unwrap_or_else(|| {
                let texture = TextureBuffer::from_memory(
                    &mut renderer,
                    &frame.pixels_rgba,
                    Fourcc::Abgr8888,
                    (frame.width as i32, frame.height as i32),
                    false,
                    1,
                    Transform::Normal,
                    None,
                )
                .expect("Failed to import cursor bitmap");
                pointer_images.push((frame, texture.clone()));
                texture
            });

        let windows = self.space.elements().cloned().collect::<Vec<_>>();

        let pointer_location = self
            .seat
            .get_pointer()
            .map(|ptr| ptr.current_location())
            .unwrap_or((0.0, 0.0).into());

        let result = render_surface(
            surface,
            &mut renderer,
            output,
            &self.space,
            &windows,
            self.dnd_icon.as_ref(),
            &mut self.cursor_status,
            &pointer_image,
            &mut udev.pointer_element,
            pointer_location,
            &self.clock,
        );

        match result {
            Ok(true) => surface.render_state = RenderState::WaitingForVblank { dirty: false },
            Ok(false) | Err(_) => surface.render_state = RenderState::Idle,
        }
    }
}

fn render_surface_for_output<'a>(
    output: &Output,
    backends: &'a mut HashMap<DrmNode, UdevBackendData>,
) -> Option<&'a mut RenderSurface> {
    let UdevOutputData { device_id, crtc } = output.user_data().get()?;

    backends
        .get_mut(device_id)
        .and_then(|device| device.surfaces.get_mut(crtc))
}

/// Render windows, layers, and everything else needed to the given [`RenderSurface`].
/// Also queues the frame for scanout.
///
/// `windows` should be provided in the order of z-rendering, top to bottom.
#[allow(clippy::too_many_arguments)]
fn render_surface(
    surface: &mut RenderSurface,
    renderer: &mut UdevRenderer<'_>,
    output: &Output,

    space: &Space<WindowElement>,
    windows: &[WindowElement],

    dnd_icon: Option<&WlSurface>,
    cursor_status: &mut CursorImageStatus,

    pointer_image: &TextureBuffer<MultiTexture>,
    pointer_element: &mut PointerElement<MultiTexture>,
    pointer_location: Point<f64, Logical>,

    clock: &Clock<Monotonic>,
) -> Result<bool, SwapBuffersError> {
    let output_render_elements = crate::render::generate_render_elements(
        output,
        renderer,
        space,
        windows,
        pointer_location,
        cursor_status,
        dnd_icon,
        // input_method,
        pointer_element,
        Some(pointer_image),
    );

    let res = render_frame(
        &mut surface.compositor,
        renderer,
        &output_render_elements,
        [0.6, 0.6, 0.6, 1.0],
    )?;

    let time = clock.now();

    if let CursorImageStatus::Surface(surf) = cursor_status {
        send_frames_surface_tree(surf, output, time, Some(Duration::ZERO), |_, _| None);
    }

    super::post_repaint(
        output,
        &res.states,
        space,
        surface
            .dmabuf_feedback
            .as_ref()
            .map(|feedback| SurfaceDmabufFeedback {
                render_feedback: &feedback.render_feedback,
                scanout_feedback: &feedback.scanout_feedback,
            }),
        time.into(),
    );

    if res.rendered {
        let output_presentation_feedback = take_presentation_feedback(output, space, &res.states);
        surface
            .compositor
            .queue_frame(Some(output_presentation_feedback))
            .map_err(SwapBuffersError::from)?;
    }

    Ok(res.rendered)
}
