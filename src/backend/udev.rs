// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    os::fd::FromRawFd,
    path::Path,
    time::Duration,
};

use anyhow::Context;
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
            damage::{self},
            element::{
                surface::WaylandSurfaceRenderElement, texture::TextureBuffer, RenderElement,
                RenderElementStates,
            },
            gles::{GlesRenderer, GlesTexture},
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer, MultiTexture},
            Bind, ExportMem, ImportDma, ImportEgl, ImportMemWl, Offscreen, Renderer,
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
        utils::{send_frames_surface_tree, OutputPresentationFeedback},
        Space,
    },
    input::pointer::CursorImageStatus,
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        ash::vk::ExtPhysicalDeviceDrmFn,
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle, RegistrationToken,
        },
        drm::{
            self,
            control::{connector, crtc, ModeTypeFlags},
            Device,
        },
        input::Libinput,
        nix::fcntl::OFlag,
        wayland_protocols::wp::{
            linux_dmabuf::zv1::server::zwp_linux_dmabuf_feedback_v1,
            presentation_time::server::wp_presentation_feedback,
        },
        wayland_server::{
            backend::GlobalId, protocol::wl_surface::WlSurface, Display, DisplayHandle,
        },
    },
    utils::{Clock, DeviceFd, IsAlive, Logical, Monotonic, Point, Transform},
    wayland::dmabuf::{DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
    xwayland::X11Surface,
};
use smithay_drm_extras::{
    drm_scanner::{DrmScanEvent, DrmScanner},
    edid::EdidInfo,
};

use crate::{
    backend::Backend,
    config::{
        api::msg::{Args, OutgoingMsg},
        ConnectorSavedState,
    },
    output::OutputName,
    render::{pointer::PointerElement, take_presentation_feedback, CustomRenderElements},
    state::{CalloopData, State, SurfaceDmabufFeedback, WithState},
    window::WindowElement,
};

use super::BackendData;

const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];
const SUPPORTED_FORMATS_8BIT_ONLY: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];

/// A [`MultiRenderer`] that uses the [`GbmGlesBackend`].
type UdevRenderer<'a, 'b> =
    MultiRenderer<'a, 'a, 'b, GbmGlesBackend<GlesRenderer>, GbmGlesBackend<GlesRenderer>>;

/// Udev state attached to each [`Output`].
#[derive(Debug, PartialEq)]
struct UdevOutputData {
    /// The GPU node
    device_id: DrmNode,
    /// The [Crtc][crtc::Handle] the output is pushing to
    crtc: crtc::Handle,
    mode: Option<drm::control::Mode>,
}

pub struct Udev {
    pub session: LibSeatSession,
    display_handle: DisplayHandle,
    pub(super) dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub(super) primary_gpu: DrmNode,
    allocator: Option<Box<dyn Allocator<Buffer = Dmabuf, Error = AnyError>>>,
    pub(super) gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer>>,
    backends: HashMap<DrmNode, UdevBackendData>,
    pointer_images: Vec<(xcursor::parser::Image, TextureBuffer<MultiTexture>)>,
    pointer_element: PointerElement<MultiTexture>,
    pointer_image: crate::cursor::Cursor,
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
        if let Err(err) =
            self.gpu_manager
                .early_import(Some(self.primary_gpu), self.primary_gpu, surface)
        {
            tracing::warn!("early buffer import failed: {}", err);
        }
    }
}

pub fn run_udev() -> anyhow::Result<()> {
    let mut event_loop = EventLoop::try_new()?;
    let display = Display::new()?;

    // Initialize session
    let (session, notifier) = LibSeatSession::new()?;

    // Get the primary gpu
    let primary_gpu = udev::primary_gpu(&session.seat())
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
    };

    let display_handle = display.handle();

    let mut state = State::init(
        Backend::Udev(data),
        display,
        event_loop.get_signal(),
        event_loop.handle(),
    )?;

    // Initialize the udev backend
    let udev_backend = UdevBackend::new(state.seat.name())?;

    // Create DrmNodes from already connected GPUs
    for (device_id, path) in udev_backend.device_list() {
        if let Err(err) = DrmNode::from_dev_id(device_id)
            .map_err(DeviceAddError::DrmNode)
            .and_then(|node| state.device_added(node, path))
        {
            tracing::error!("Skipping device {device_id}: {err}");
        }
    }

    let udev = state.backend.udev_mut();

    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, data| match event {
            // GPU connected
            UdevEvent::Added { device_id, path } => {
                if let Err(err) = DrmNode::from_dev_id(device_id)
                    .map_err(DeviceAddError::DrmNode)
                    .and_then(|node| data.state.device_added(node, &path))
                {
                    tracing::error!("Skipping device {device_id}: {err}");
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    data.state.device_changed(node)
                }
            }
            // GPU disconnected
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    data.state.device_removed(node)
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
        .insert_source(libinput_backend, move |event, _, data| {
            data.state.apply_libinput_settings(&event);
            data.state.process_input_event(event);
        });

    if let Err(err) = insert_ret {
        anyhow::bail!("Failed to insert libinput_backend into event loop: {err}");
    }

    event_loop
        .handle()
        .insert_source(notifier, move |event, _, data| {
            let udev = data.state.backend.udev_mut();

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
                        tracing::error!("Failed to resume libinput context: {:?}", err);
                    }
                    for backend in udev.backends.values_mut() {
                        backend.drm.activate();
                        for surface in backend.surfaces.values_mut() {
                            if let Err(err) = surface.compositor.surface().reset_state() {
                                tracing::warn!("Failed to reset drm surface state: {}", err);
                            }
                            // reset the buffers after resume to trigger a full redraw
                            // this is important after a vt switch as the primary plane
                            // has no content and damage tracking may prevent a redraw
                            // otherwise
                            surface.compositor.reset_buffers();
                        }
                    }

                    for output in data.state.space.outputs().cloned().collect::<Vec<_>>() {
                        data.state
                            .loop_handle
                            .insert_idle(move |data| data.state.render_surface(&output));
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
                    tracing::warn!("Failed to create vulkan allocator: {}", err);
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
        Err(err) => tracing::error!(?err, "Failed to initialize EGL hardware-acceleration"),
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
        tracing::error!("Failed to start XWayland: {err}");
    }

    event_loop.run(
        Some(Duration::from_micros(((1.0 / 144.0) * 1000000.0) as u64)),
        &mut CalloopData {
            state,
            display_handle,
        },
        |data| {
            data.state.space.refresh();
            data.state.popup_manager.cleanup();
            data.display_handle
                .flush_clients()
                .expect("failed to flush_clients");
        },
    )?;

    Ok(())
}

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
    gpu_manager: &mut GpuManager<GbmGlesBackend<GlesRenderer>>,
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
fn render_frame<R, E, Target>(
    compositor: &mut GbmDrmCompositor,
    renderer: &mut R,
    elements: &[E],
    clear_color: [f32; 4],
) -> Result<SurfaceCompositorRenderResult, SwapBuffersError>
where
    R: Renderer + Bind<Dmabuf> + Bind<Target> + Offscreen<Target> + ExportMem,
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
                rendered: render_frame_result.damage.is_some(),
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
                OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_NONBLOCK,
            )
            .map_err(DeviceAddError::DeviceOpen)?;

        let fd = DrmDeviceFd::new(unsafe { DeviceFd::from_raw_fd(fd) });

        let (drm, notifier) =
            DrmDevice::new(fd.clone(), true).map_err(DeviceAddError::DrmDevice)?;
        let gbm = GbmDevice::new(fd).map_err(DeviceAddError::GbmDevice)?;

        let registration_token = self
            .loop_handle
            .insert_source(
                notifier,
                move |event, metadata, data: &mut CalloopData| match event {
                    DrmEvent::VBlank(crtc) => {
                        data.state.frame_finish(node, crtc, metadata);
                    }
                    DrmEvent::Error(error) => {
                        tracing::error!("{:?}", error);
                    }
                },
            )
            .expect("failed to insert drm notifier into event loop");

        let render_node = EGLDevice::device_for_display(
            &EGLDisplay::new(gbm.clone()).expect("failed to create EGLDisplay"),
        )
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
    // TODO: better edid info from cosmic-comp
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

        tracing::debug!(clock = ?drm_mode.clock(), hsync = ?drm_mode.hsync(), vsync = ?drm_mode.vsync());

        let surface = match device
            .drm
            .create_surface(crtc, drm_mode, &[connector.handle()])
        {
            Ok(surface) => surface,
            Err(err) => {
                tracing::warn!("Failed to create drm surface: {}", err);
                return;
            }
        };

        let output_name = format!(
            "{}-{}",
            connector.interface().as_str(),
            connector.interface_id()
        );

        let (make, model) = EdidInfo::for_connector(&device.drm, connector.handle())
            .map(|info| (info.manufacturer, info.model))
            .unwrap_or_else(|| ("Unknown".into(), "Unknown".into()));

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
                subpixel: Subpixel::Unknown,
                make,
                model,
            },
        );
        let global = output.create_global::<State>(&udev.display_handle);

        self.focus_state.focused_output = Some(output.clone());

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

        // The preferred mode with the highest refresh rate
        // Logic from niri
        let mode = connector
            .modes()
            .iter()
            .max_by(|mode1, mode2| {
                let mode1_preferred = mode1.mode_type().contains(ModeTypeFlags::PREFERRED);
                let mode2_preferred = mode2.mode_type().contains(ModeTypeFlags::PREFERRED);

                use std::cmp::Ordering;

                match (mode1_preferred, mode2_preferred) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => mode1.vrefresh().cmp(&mode2.vrefresh()),
                }
            })
            .copied();

        output.user_data().insert_if_missing(|| UdevOutputData {
            crtc,
            device_id: node,
            mode,
        });

        let allocator = GbmAllocator::new(
            device.gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );

        let color_formats = if std::env::var("ANVIL_DISABLE_10BIT").is_ok() {
            SUPPORTED_FORMATS_8BIT_ONLY
        } else {
            SUPPORTED_FORMATS
        };

        let compositor = {
            let driver = match device.drm.get_driver() {
                Ok(driver) => driver,
                Err(err) => {
                    tracing::warn!("Failed to query drm driver: {}", err);
                    return;
                }
            };

            let mut planes = surface.planes().clone();

            // Using an overlay plane on a nvidia card breaks
            if driver
                .name()
                .to_string_lossy()
                .to_lowercase()
                .contains("nvidia")
                || driver
                    .description()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("nvidia")
            {
                planes.overlay = vec![];
            }

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
                    tracing::warn!("Failed to create drm compositor: {}", err);
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
        };

        device.surfaces.insert(crtc, surface);

        self.schedule_initial_render(node, crtc, self.loop_handle.clone());

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

            output.with_state(|state| state.tags = tags.clone());
        } else {
            // Run any output callbacks
            let clone = output.clone();
            self.schedule(
                |dt| dt.state.api_state.stream.is_some(),
                move |dt| {
                    let stream = dt
                        .state
                        .api_state
                        .stream
                        .as_ref()
                        .expect("Stream doesn't exist");
                    let mut stream = stream.lock().expect("Couldn't lock stream");
                    for callback_id in dt.state.config.output_callback_ids.iter() {
                        crate::config::api::send_to_client(
                            &mut stream,
                            &OutgoingMsg::CallCallback {
                                callback_id: *callback_id,
                                args: Some(Args::ConnectForAllOutputs {
                                    output_name: clone.name(),
                                }),
                            },
                        )
                        .expect("Send to client failed");
                    }
                },
            );
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
    fn frame_finish(
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

        let schedule_render = match surface
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

                true
            }
            Err(err) => {
                tracing::warn!("Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => true,
                    // If the device has been deactivated do not reschedule, this will be done
                    // by session resume
                    SwapBuffersError::TemporaryFailure(err)
                        if matches!(
                            err.downcast_ref::<DrmError>(),
                            Some(&DrmError::DeviceInactive)
                        ) =>
                    {
                        false
                    }
                    SwapBuffersError::TemporaryFailure(err) => matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::Access {
                            source: drm::SystemError::PermissionDenied,
                            ..
                        })
                    ),
                    SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
                }
            }
        };

        if schedule_render {
            // Anvil had some stuff here about delaying a render to reduce latency,
            // but it introduces visible hitching when scrolling, so I'm removing it here.
            //
            // If latency is a problem then future me can deal with it :)
            self.loop_handle.insert_idle(move |data| {
                data.state.render_surface(&output);
            });
        }
    }

    /// Render to the [`RenderSurface`] associated with the given `output`.
    fn render_surface(&mut self, output: &Output) {
        let udev = self.backend.udev_mut();

        let Some(UdevOutputData {
            device_id,
            crtc,
            mode: _,
        }) = output.user_data().get()
        else {
            return;
        };

        let Some(surface) = udev
            .backends
            .get_mut(device_id)
            .and_then(|device| device.surfaces.get_mut(crtc))
        else {
            return;
        };

        // TODO get scale from the rendersurface when supporting HiDPI
        let frame = udev.pointer_image.get_image(
            1, /*scale*/
            self.clock
                .now()
                .try_into()
                .expect("failed to convert time into duration"),
        );

        let render_node = surface.render_node;
        let primary_gpu = udev.primary_gpu;
        let mut renderer = if primary_gpu == render_node {
            udev.gpu_manager.single_renderer(&render_node)
        } else {
            let format = surface.compositor.format();
            udev.gpu_manager.renderer(
                &primary_gpu,
                &render_node,
                udev
                    .allocator
                    .as_mut()
                    // TODO: We could build some kind of `GLAllocator` using Renderbuffers in theory for this case.
                    //  That would work for memcpy's of offscreen contents.
                    .expect("We need an allocator for multigpu systems")
                    .as_mut(),
                format,
            )
        }
        .expect("failed to create MultiRenderer");

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

        let windows = self
            .focus_state
            .focus_stack
            .iter()
            .filter(|win| win.alive())
            .cloned()
            .collect::<Vec<_>>();

        let result = render_surface(
            surface,
            &mut renderer,
            output,
            &self.space,
            &windows,
            &self.override_redirect_windows,
            self.dnd_icon.as_ref(),
            &mut self.cursor_status,
            &pointer_image,
            &mut udev.pointer_element,
            self.pointer_location,
            &self.clock,
        );

        let reschedule = match &result {
            Ok(has_rendered) => !has_rendered,
            Err(err) => {
                tracing::warn!("Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => false,
                    SwapBuffersError::TemporaryFailure(err) => !matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::DeviceInactive)
                            | Some(&DrmError::Access {
                                source: drm::SystemError::PermissionDenied,
                                ..
                            })
                    ),
                    SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
                }
            }
        };

        if reschedule {
            let Some(data) = output.user_data().get::<UdevOutputData>() else {
                unreachable!()
            };

            // Literally no idea if this refresh time calculation is doing anything, but we're
            // gonna keep it here because I already added the stuff for it
            let refresh_time = if let Some(mode) = data.mode {
                self::utils::refresh_time(mode)
            } else {
                let output_refresh = match output.current_mode() {
                    Some(mode) => mode.refresh,
                    None => {
                        return;
                    }
                };
                Duration::from_millis((1_000_000f32 / output_refresh as f32) as u64)
            };

            // If reschedule is true we either hit a temporary failure or more likely rendering
            // did not cause any damage on the output. In this case we just re-schedule a repaint
            // after approx. one frame to re-test for damage.
            tracing::trace!(
                "reschedule repaint timer with delay {:?} on {:?}",
                refresh_time,
                crtc,
            );
            let timer = Timer::from_duration(refresh_time);
            let output = output.clone();
            self.loop_handle
                .insert_source(timer, move |_, _, data| {
                    data.state.render_surface(&output);
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }
    }

    /// Do an initial render that renders nothing to the screen.
    ///
    /// If that render failed, schedule another one.
    fn schedule_initial_render(
        &mut self,
        node: DrmNode,
        crtc: crtc::Handle,
        evt_handle: LoopHandle<'static, CalloopData>,
    ) {
        let udev = self.backend.udev_mut();

        let Some(surface) = udev
            .backends
            .get_mut(&node)
            .and_then(|device| device.surfaces.get_mut(&crtc))
        else {
            return;
        };

        let node = surface.render_node;
        let result = {
            let mut renderer = udev
                .gpu_manager
                .single_renderer(&node)
                .expect("failed to create MultiRenderer");
            initial_render(surface, &mut renderer)
        };

        if let Err(err) = result {
            match err {
                SwapBuffersError::AlreadySwapped => {}
                SwapBuffersError::TemporaryFailure(err) => {
                    // TODO dont reschedule after 3(?) retries
                    tracing::warn!("Failed to submit page_flip: {}", err);
                    let handle = evt_handle.clone();
                    evt_handle.insert_idle(move |data| {
                        data.state.schedule_initial_render(node, crtc, handle)
                    });
                }
                SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
            }
        }
    }
}

/// Render windows, layers, and everything else needed to the given [`RenderSurface`].
/// Also queues the frame for scanout.
#[allow(clippy::too_many_arguments)]
fn render_surface<'a>(
    surface: &'a mut RenderSurface,
    renderer: &mut UdevRenderer<'a, '_>,
    output: &Output,

    space: &Space<WindowElement>,
    windows: &[WindowElement],
    override_redirect_windows: &[X11Surface],

    dnd_icon: Option<&WlSurface>,
    cursor_status: &mut CursorImageStatus,

    pointer_image: &TextureBuffer<MultiTexture>,
    pointer_element: &mut PointerElement<MultiTexture>,
    pointer_location: Point<f64, Logical>,

    clock: &Clock<Monotonic>,
) -> Result<bool, SwapBuffersError> {
    let pending_wins = windows
        .iter()
        .filter(|win| win.alive())
        .filter(|win| {
            let pending_size = if let WindowElement::Wayland(win) = win {
                let current_state = win.toplevel().current_state();
                win.toplevel()
                    .with_pending_state(|state| state.size != current_state.size)
            } else {
                false
            };
            pending_size || win.with_state(|state| !state.loc_request_state.is_idle())
        })
        .map(|win| {
            (
                win.class().unwrap_or("None".to_string()),
                win.title().unwrap_or("None".to_string()),
                win.with_state(|state| state.loc_request_state.clone()),
            )
        })
        .collect::<Vec<_>>();

    if !pending_wins.is_empty() {
        tracing::debug!("Skipping frame, waiting on {pending_wins:?}");
        for win in windows.iter() {
            win.send_frame(output, clock.now(), Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }

        surface
            .compositor
            .queue_frame(None)
            .map_err(Into::<SwapBuffersError>::into)?;

        // TODO: still draw the cursor here

        return Ok(true);
    }

    let output_render_elements = crate::render::generate_render_elements(
        output,
        renderer,
        space,
        windows,
        override_redirect_windows,
        pointer_location,
        cursor_status,
        dnd_icon,
        // input_method,
        pointer_element,
        Some(pointer_image),
    );

    let res = render_frame::<_, _, GlesTexture>(
        &mut surface.compositor,
        renderer,
        &output_render_elements,
        [0.6, 0.6, 0.6, 1.0],
    )?;

    let time = clock.now();

    // Send frames to the cursor surface to get it to update correctly
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

/// Renders nothing to the given [`RenderSurface`].
fn initial_render(
    surface: &mut RenderSurface,
    renderer: &mut UdevRenderer<'_, '_>,
) -> Result<(), SwapBuffersError> {
    render_frame::<_, CustomRenderElements<_, WaylandSurfaceRenderElement<_>>, GlesTexture>(
        &mut surface.compositor,
        renderer,
        &[],
        [0.6, 0.6, 0.6, 1.0],
    )?;
    surface.compositor.queue_frame(None)?;
    surface.compositor.reset_buffers();

    Ok(())
}
