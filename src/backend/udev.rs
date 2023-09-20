// SPDX-License-Identifier: GPL-3.0-or-later

// from anvil
// TODO: figure out what this stuff does

#![allow(clippy::unwrap_used)] // I don't know what this stuff does yet
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    os::fd::FromRawFd,
    path::Path,
    time::Duration,
};

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
            DrmNode, DrmSurface, GbmBufferedSurface, NodeType,
        },
        egl::{self, EGLDevice, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage::{self, OutputDamageTracker},
            element::{texture::TextureBuffer, RenderElement, RenderElementStates},
            gles::{GlesRenderer, GlesTexture},
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer, MultiTexture},
            sync::SyncPoint,
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
        gbm,
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
    utils::{Clock, DeviceFd, IsAlive, Logical, Monotonic, Physical, Point, Rectangle, Transform},
    wayland::{
        dmabuf::{DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState},
        input_method::{InputMethodHandle, InputMethodSeat},
    },
};
use smithay_drm_extras::{
    drm_scanner::{DrmScanEvent, DrmScanner},
    edid::EdidInfo,
};

use crate::{
    config::api::msg::{Args, OutgoingMsg},
    render::{pointer::PointerElement, take_presentation_feedback, CustomRenderElements},
    state::{Backend, CalloopData, State, SurfaceDmabufFeedback, WithState},
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

type UdevRenderer<'a, 'b> =
    MultiRenderer<'a, 'a, 'b, GbmGlesBackend<GlesRenderer>, GbmGlesBackend<GlesRenderer>>;

#[derive(Debug, PartialEq)]
struct UdevOutputId {
    device_id: DrmNode,
    crtc: crtc::Handle,
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

impl BackendData for Udev {
    fn seat_name(&self) -> String {
        self.session.seat()
    }

    fn reset_buffers(&mut self, output: &Output) {
        if let Some(id) = output.user_data().get::<UdevOutputId>() {
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
    let mut event_loop = EventLoop::try_new().unwrap();
    let mut display = Display::new().unwrap();

    // Initialize session
    let (session, notifier) = LibSeatSession::new()?;

    // Initialize the compositor
    let primary_gpu = if let Ok(var) = std::env::var("ANVIL_DRM_DEVICE") {
        DrmNode::from_path(var).expect("Invalid drm device path")
    } else {
        udev::primary_gpu(&session.seat())
            .unwrap()
            .and_then(|x| {
                DrmNode::from_path(x)
                    .ok()?
                    .node_with_type(NodeType::Render)?
                    .ok()
            })
            .unwrap_or_else(|| {
                udev::all_gpus(session.seat())
                    .unwrap()
                    .into_iter()
                    .find_map(|x| DrmNode::from_path(x).ok())
                    .expect("No GPU!")
            })
    };
    tracing::info!("Using {} as primary gpu.", primary_gpu);

    let gpu_manager = GpuManager::new(GbmGlesBackend::default()).unwrap();

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

    let mut state = State::init(
        Backend::Udev(data),
        &mut display,
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

    let Backend::Udev(backend) = &mut state.backend else {
        unreachable!()
    };

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
        .unwrap();

    // Initialize libinput backend
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        backend.session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(state.seat.name())
        .unwrap();
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // Bind all our objects that get driven by the event loop
    let insert_ret = event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, data| {
            // println!("event: {:?}", event);
            data.state.process_input_event(event);
        });

    if let Err(err) = insert_ret {
        anyhow::bail!("Failed to insert libinput_backend into event loop: {err}");
    }

    let handle = event_loop.handle();
    event_loop
        .handle()
        .insert_source(notifier, move |event, &mut (), data| {
            let Backend::Udev(backend) = &mut data.state.backend else {
                unreachable!()
            };
            match event {
                session::Event::PauseSession => {
                    libinput_context.suspend();
                    tracing::info!("pausing session");

                    for backend in backend.backends.values_mut() {
                        backend.drm.pause();
                    }
                }
                session::Event::ActivateSession => {
                    tracing::info!("resuming session");

                    if let Err(err) = libinput_context.resume() {
                        tracing::error!("Failed to resume libinput context: {:?}", err);
                    }
                    for (node, backend) in backend
                        .backends
                        .iter_mut()
                        .map(|(handle, backend)| (*handle, backend))
                    {
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
                        handle.insert_idle(move |data| data.state.render(node, None));
                    }
                }
            }
        })
        .unwrap();

    state.shm_state.update_formats(
        backend
            .gpu_manager
            .single_renderer(&primary_gpu)
            .unwrap()
            .shm_formats(),
    );

    let skip_vulkan = std::env::var("ANVIL_NO_VULKAN")
        .map(|x| {
            x == "1"
                || x.to_lowercase() == "true"
                || x.to_lowercase() == "yes"
                || x.to_lowercase() == "y"
        })
        .unwrap_or(false);

    if !skip_vulkan {
        if let Ok(instance) = vulkan::Instance::new(Version::VERSION_1_2, None) {
            if let Some(physical_device) =
                PhysicalDevice::enumerate(&instance)
                    .ok()
                    .and_then(|devices| {
                        devices
                            .filter(|phd| phd.has_device_extension(ExtPhysicalDeviceDrmFn::name()))
                            .find(|phd| {
                                phd.primary_node().unwrap() == Some(primary_gpu)
                                    || phd.render_node().unwrap() == Some(primary_gpu)
                            })
                    })
            {
                match VulkanAllocator::new(
                    &physical_device,
                    ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
                ) {
                    Ok(allocator) => {
                        backend.allocator = Some(Box::new(DmabufAllocator(allocator))
                            as Box<dyn Allocator<Buffer = Dmabuf, Error = AnyError>>);
                    }
                    Err(err) => {
                        tracing::warn!("Failed to create vulkan allocator: {}", err);
                    }
                }
            }
        }
    }

    if backend.allocator.is_none() {
        tracing::info!("No vulkan allocator found, using GBM.");
        let gbm = backend
            .backends
            .get(&primary_gpu)
            // If the primary_gpu failed to initialize, we likely have a kmsro device
            .or_else(|| backend.backends.values().next())
            // Don't fail, if there is no allocator. There is a chance, that this a single gpu system and we don't need one.
            .map(|backend| backend.gbm.clone());
        backend.allocator = gbm.map(|gbm| {
            Box::new(DmabufAllocator(GbmAllocator::new(
                gbm,
                GbmBufferFlags::RENDERING,
            ))) as Box<_>
        });
    }

    #[cfg_attr(not(feature = "egl"), allow(unused_mut))]
    let mut renderer = backend.gpu_manager.single_renderer(&primary_gpu).unwrap();

    {
        tracing::info!(
            ?primary_gpu,
            "Trying to initialize EGL Hardware Acceleration",
        );
        match renderer.bind_wl_display(&display.handle()) {
            Ok(_) => tracing::info!("EGL hardware-acceleration enabled"),
            Err(err) => tracing::info!(?err, "Failed to initialize EGL hardware-acceleration"),
        }
    }

    // init dmabuf support with format list from our primary gpu
    let dmabuf_formats = renderer.dmabuf_formats().collect::<Vec<_>>();
    let default_feedback = DmabufFeedbackBuilder::new(primary_gpu.dev_id(), dmabuf_formats)
        .build()
        .unwrap();
    let mut dmabuf_state = DmabufState::new();
    let global = dmabuf_state
        .create_global_with_default_feedback::<State>(&display.handle(), &default_feedback);
    backend.dmabuf_state = Some((dmabuf_state, global));

    let gpu_manager = &mut backend.gpu_manager;
    backend.backends.values_mut().for_each(|backend_data| {
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
        Some(Duration::from_millis(1)),
        &mut CalloopData { state, display },
        |data| {
            data.state.space.refresh();
            data.state.popup_manager.cleanup();
            data.display.flush_clients().unwrap();
        },
    )?;

    Ok(())
}

struct UdevBackendData {
    surfaces: HashMap<crtc::Handle, SurfaceData>,
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
    composition: &SurfaceComposition,
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
        .unwrap();

    let scanout_feedback = builder
        .add_preference_tranche(
            surface.device_fd().dev_id().unwrap(),
            Some(zwp_linux_dmabuf_feedback_v1::TrancheFlags::Scanout),
            planes_formats,
        )
        .add_preference_tranche(render_node.dev_id(), None, render_formats)
        .build()
        .unwrap();

    Some(DrmSurfaceDmabufFeedback {
        render_feedback,
        scanout_feedback,
    })
}

struct DrmSurfaceDmabufFeedback {
    render_feedback: DmabufFeedback,
    scanout_feedback: DmabufFeedback,
}

struct SurfaceData {
    global: Option<GlobalId>,
    display_handle: DisplayHandle,
    device_id: DrmNode,
    render_node: DrmNode,
    compositor: SurfaceComposition,
    dmabuf_feedback: Option<DrmSurfaceDmabufFeedback>,
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        if let Some(global) = self.global.take() {
            self.display_handle.remove_global::<State>(global);
        }
    }
}

type RenderSurface =
    GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, Option<OutputPresentationFeedback>>;

type GbmDrmCompositor = DrmCompositor<
    GbmAllocator<DrmDeviceFd>,
    GbmDevice<DrmDeviceFd>,
    Option<OutputPresentationFeedback>,
    DrmDeviceFd,
>;

enum SurfaceComposition {
    Surface {
        surface: RenderSurface,
        damage_tracker: OutputDamageTracker,
    },
    Compositor(GbmDrmCompositor),
}

struct SurfaceCompositorRenderResult {
    rendered: bool,
    states: RenderElementStates,
    sync: Option<SyncPoint>,
    damage: Option<Vec<Rectangle<i32, Physical>>>,
}

impl SurfaceComposition {
    fn frame_submitted(&mut self) -> Result<Option<OutputPresentationFeedback>, SwapBuffersError> {
        match self {
            SurfaceComposition::Surface { surface, .. } => surface
                .frame_submitted()
                .map(Option::flatten)
                .map_err(SwapBuffersError::from),
            SurfaceComposition::Compositor(comp) => comp
                .frame_submitted()
                .map(Option::flatten)
                .map_err(SwapBuffersError::from),
        }
    }

    fn format(&self) -> gbm::Format {
        match self {
            SurfaceComposition::Surface { surface, .. } => surface.format(),
            SurfaceComposition::Compositor(comp) => comp.format(),
        }
    }

    fn surface(&self) -> &DrmSurface {
        match self {
            SurfaceComposition::Compositor(c) => c.surface(),
            SurfaceComposition::Surface { surface, .. } => surface.surface(),
        }
    }

    fn reset_buffers(&mut self) {
        match self {
            SurfaceComposition::Compositor(c) => c.reset_buffers(),
            SurfaceComposition::Surface { surface, .. } => surface.reset_buffers(),
        }
    }

    fn queue_frame(
        &mut self,
        sync: Option<SyncPoint>,
        damage: Option<Vec<Rectangle<i32, Physical>>>,
        user_data: Option<OutputPresentationFeedback>,
    ) -> Result<(), SwapBuffersError> {
        match self {
            SurfaceComposition::Surface { surface, .. } => surface
                .queue_buffer(sync, damage, user_data)
                .map_err(Into::<SwapBuffersError>::into),
            SurfaceComposition::Compositor(c) => c
                .queue_frame(user_data)
                .map_err(Into::<SwapBuffersError>::into),
        }
    }

    fn render_frame<R, E, Target>(
        &mut self,
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
        match self {
            SurfaceComposition::Surface {
                surface,
                damage_tracker,
            } => {
                let (dmabuf, age) = surface
                    .next_buffer()
                    .map_err(Into::<SwapBuffersError>::into)?;
                renderer
                    .bind(dmabuf)
                    .map_err(Into::<SwapBuffersError>::into)?;
                let current_debug_flags = renderer.debug_flags();

                tracing::info!("surface damage_tracker render_output");

                let res = damage_tracker
                    .render_output(renderer, age.into(), elements, clear_color)
                    .map(|res| {
                        res.sync.wait(); // feature flag here
                        let rendered = res.damage.is_some();
                        SurfaceCompositorRenderResult {
                            rendered,
                            damage: res.damage,
                            states: res.states,
                            sync: rendered.then_some(res.sync),
                        }
                    })
                    .map_err(|err| match err {
                        damage::Error::Rendering(err) => err.into(),
                        _ => unreachable!(),
                    });
                renderer.set_debug_flags(current_debug_flags);
                res
            }
            SurfaceComposition::Compositor(compositor) => compositor
                .render_frame(renderer, elements, clear_color)
                .map(|render_frame_result| {
                    // feature flag here
                    if let PrimaryPlaneElement::Swapchain(element) =
                        render_frame_result.primary_element
                    {
                        element.sync.wait();
                    }
                    SurfaceCompositorRenderResult {
                        rendered: render_frame_result.damage.is_some(),
                        states: render_frame_result.states,
                        sync: None,
                        damage: None,
                    }
                })
                .map_err(|err| match err {
                    smithay::backend::drm::compositor::RenderFrameError::PrepareFrame(err) => {
                        err.into()
                    }
                    smithay::backend::drm::compositor::RenderFrameError::RenderFrame(
                        damage::Error::Rendering(err),
                    ) => err.into(),
                    _ => unreachable!(),
                }),
        }
    }
}

impl State {
    fn device_added(&mut self, node: DrmNode, path: &Path) -> Result<(), DeviceAddError> {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        // Try to open the device
        let fd = backend
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
            .unwrap();

        let render_node = EGLDevice::device_for_display(&EGLDisplay::new(gbm.clone()).unwrap())
            .ok()
            .and_then(|x| x.try_get_render_node().ok().flatten())
            .unwrap_or(node);

        backend
            .gpu_manager
            .as_mut()
            .add_node(render_node, gbm.clone())
            .map_err(DeviceAddError::AddNode)?;

        backend.backends.insert(
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

    fn connector_connected(
        &mut self,
        node: DrmNode,
        connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device = if let Some(device) = backend.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        let mut renderer = backend
            .gpu_manager
            .single_renderer(&device.render_node)
            .unwrap();
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
        let output = Output::new(
            output_name,
            PhysicalProperties {
                size: (phys_w as i32, phys_h as i32).into(),
                subpixel: Subpixel::Unknown,
                make,
                model,
            },
        );
        let global = output.create_global::<State>(&backend.display_handle);

        self.focus_state.focused_output = Some(output.clone());

        let x = self.space.outputs().fold(0, |acc, o| {
            acc + self.space.output_geometry(o).unwrap().size.w
        });
        let position = (x, 0).into();

        output.set_preferred(wl_mode);
        output.change_current_state(Some(wl_mode), None, None, Some(position));
        self.space.map_output(&output, position);

        output.user_data().insert_if_missing(|| UdevOutputId {
            crtc,
            device_id: node,
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

        let compositor = if std::env::var("ANVIL_DISABLE_DRM_COMPOSITOR").is_ok() {
            let gbm_surface =
                match GbmBufferedSurface::new(surface, allocator, color_formats, render_formats) {
                    Ok(renderer) => renderer,
                    Err(err) => {
                        tracing::warn!("Failed to create rendering surface: {}", err);
                        return;
                    }
                };
            SurfaceComposition::Surface {
                surface: gbm_surface,
                damage_tracker: OutputDamageTracker::from_output(&output),
            }
        } else {
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

            let compositor = match DrmCompositor::new(
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
            };
            SurfaceComposition::Compositor(compositor)
        };

        let dmabuf_feedback = get_surface_dmabuf_feedback(
            backend.primary_gpu,
            device.render_node,
            &mut backend.gpu_manager,
            &compositor,
        );

        let surface = SurfaceData {
            display_handle: backend.display_handle.clone(),
            device_id: node,
            render_node: device.render_node,
            global: Some(global),
            compositor,
            dmabuf_feedback,
        };

        device.surfaces.insert(crtc, surface);

        self.schedule_initial_render(node, crtc, self.loop_handle.clone());

        // Run any connected callbacks
        {
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

    fn connector_disconnected(
        &mut self,
        node: DrmNode,
        _connector: connector::Info,
        crtc: crtc::Handle,
    ) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device = if let Some(device) = backend.backends.get_mut(&node) {
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
                    .get::<UdevOutputId>()
                    .map(|id| id.device_id == node && id.crtc == crtc)
                    .unwrap_or(false)
            })
            .cloned();

        if let Some(output) = output {
            self.space.unmap_output(&output);
        }
    }

    fn device_changed(&mut self, node: DrmNode) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device = if let Some(device) = backend.backends.get_mut(&node) {
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

        // fixup window coordinates
        // crate::shell::fixup_positions(&mut self.space);
    }

    fn device_removed(&mut self, node: DrmNode) {
        let crtcs = {
            let Backend::Udev(backend) = &mut self.backend else {
                unreachable!()
            };

            let Some(device) = backend.backends.get_mut(&node) else {
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

        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        // drop the backends on this side
        if let Some(backend_data) = backend.backends.remove(&node) {
            backend
                .gpu_manager
                .as_mut()
                .remove_node(&backend_data.render_node);

            self.loop_handle.remove(backend_data.registration_token);

            tracing::debug!("Dropping device");
        }

        // crate::shell::fixup_positions(&mut self.space);
    }

    fn frame_finish(
        &mut self,
        dev_id: DrmNode,
        crtc: crtc::Handle,
        metadata: &mut Option<DrmEventMetadata>,
    ) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device_backend = match backend.backends.get_mut(&dev_id) {
            Some(backend) => backend,
            None => {
                tracing::error!("Trying to finish frame on non-existent backend {}", dev_id);
                return;
            }
        };

        let surface = match device_backend.surfaces.get_mut(&crtc) {
            Some(surface) => surface,
            None => {
                tracing::error!("Trying to finish frame on non-existent crtc {:?}", crtc);
                return;
            }
        };

        let output = if let Some(output) = self.space.outputs().find(|o| {
            o.user_data().get::<UdevOutputId>()
                == Some(&UdevOutputId {
                    device_id: surface.device_id,
                    crtc,
                })
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        let schedule_render = match surface
            .compositor
            .frame_submitted()
            .map_err(Into::<SwapBuffersError>::into)
        {
            Ok(user_data) => {
                if let Some(mut feedback) = user_data {
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
            let output_refresh = match output.current_mode() {
                Some(mode) => mode.refresh,
                None => return,
            };
            // What are we trying to solve by introducing a delay here:
            //
            // Basically it is all about latency of client provided buffers.
            // A client driven by frame callbacks will wait for a frame callback
            // to repaint and submit a new buffer. As we send frame callbacks
            // as part of the repaint in the compositor the latency would always
            // be approx. 2 frames. By introducing a delay before we repaint in
            // the compositor we can reduce the latency to approx. 1 frame + the
            // remaining duration from the repaint to the next VBlank.
            //
            // With the delay it is also possible to further reduce latency if
            // the client is driven by presentation feedback. As the presentation
            // feedback is directly sent after a VBlank the client can submit a
            // new buffer during the repaint delay that can hit the very next
            // VBlank, thus reducing the potential latency to below one frame.
            //
            // Choosing a good delay is a topic on its own so we just implement
            // a simple strategy here. We just split the duration between two
            // VBlanks into two steps, one for the client repaint and one for the
            // compositor repaint. Theoretically the repaint in the compositor should
            // be faster so we give the client a bit more time to repaint. On a typical
            // modern system the repaint in the compositor should not take more than 2ms
            // so this should be safe for refresh rates up to at least 120 Hz. For 120 Hz
            // this results in approx. 3.33ms time for repainting in the compositor.
            // A too big delay could result in missing the next VBlank in the compositor.
            //
            // A more complete solution could work on a sliding window analyzing past repaints
            // and do some prediction for the next repaint.
            let repaint_delay =
                Duration::from_millis(((1_000_000f32 / output_refresh as f32) * 0.6f32) as u64);

            let timer = if backend.primary_gpu != surface.render_node {
                // However, if we need to do a copy, that might not be enough.
                // (And without actual comparision to previous frames we cannot really know.)
                // So lets ignore that in those cases to avoid thrashing performance.
                tracing::trace!("scheduling repaint timer immediately on {:?}", crtc);
                Timer::immediate()
            } else {
                tracing::trace!(
                    "scheduling repaint timer with delay {:?} on {:?}",
                    repaint_delay,
                    crtc
                );
                Timer::from_duration(repaint_delay)
            };

            self.loop_handle
                .insert_source(timer, move |_, _, data| {
                    data.state.render(dev_id, Some(crtc));
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }
    }

    // If crtc is `Some()`, render it, else render all crtcs
    fn render(&mut self, node: DrmNode, crtc: Option<crtc::Handle>) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device_backend = match backend.backends.get_mut(&node) {
            Some(backend) => backend,
            None => {
                tracing::error!("Trying to render on non-existent backend {}", node);
                return;
            }
        };

        if let Some(crtc) = crtc {
            self.render_surface(node, crtc);
        } else {
            let crtcs: Vec<_> = device_backend.surfaces.keys().copied().collect();
            for crtc in crtcs {
                self.render_surface(node, crtc);
            }
        };
    }

    fn render_surface(&mut self, node: DrmNode, crtc: crtc::Handle) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let Some(device) = backend.backends.get_mut(&node) else {
            return;
        };

        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };

        // TODO get scale from the rendersurface when supporting HiDPI
        let frame = backend
            .pointer_image
            .get_image(1 /*scale*/, self.clock.now().try_into().unwrap());

        let render_node = surface.render_node;
        let primary_gpu = backend.primary_gpu;
        let mut renderer = if primary_gpu == render_node {
            backend.gpu_manager.single_renderer(&render_node)
        } else {
            let format = surface.compositor.format();
            backend.gpu_manager.renderer(
                &primary_gpu,
                &render_node,
                backend
                    .allocator
                    .as_mut()
                    // TODO: We could build some kind of `GLAllocator` using Renderbuffers in theory for this case.
                    //  That would work for memcpy's of offscreen contents.
                    .expect("We need an allocator for multigpu systems")
                    .as_mut(),
                format,
            )
        }
        .unwrap();

        let pointer_images = &mut backend.pointer_images;
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

        let output = if let Some(output) = self.space.outputs().find(|o| {
            o.user_data().get::<UdevOutputId>()
                == Some(&UdevOutputId {
                    device_id: surface.device_id,
                    crtc,
                })
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        let windows = self
            .focus_state
            .focus_stack
            .iter()
            .filter(|win| win.alive())
            .cloned()
            .collect::<Vec<_>>();

        let result = render_surface(
            &mut self.cursor_status,
            &self.space,
            &windows,
            self.dnd_icon.as_ref(),
            &self.focus_state.focus_stack,
            surface,
            &mut renderer,
            &output,
            self.seat.input_method(),
            &pointer_image,
            &mut backend.pointer_element,
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
            let output_refresh = match output.current_mode() {
                Some(mode) => mode.refresh,
                None => {
                    return;
                }
            };
            // If reschedule is true we either hit a temporary failure or more likely rendering
            // did not cause any damage on the output. In this case we just re-schedule a repaint
            // after approx. one frame to re-test for damage.
            let reschedule_duration =
                Duration::from_millis((1_000_000f32 / output_refresh as f32) as u64);
            tracing::trace!(
                "reschedule repaint timer with delay {:?} on {:?}",
                reschedule_duration,
                crtc,
            );
            let timer = Timer::from_duration(reschedule_duration);
            self.loop_handle
                .insert_source(timer, move |_, _, data| {
                    data.state.render(node, Some(crtc));
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }
    }

    fn schedule_initial_render(
        &mut self,
        node: DrmNode,
        crtc: crtc::Handle,
        evt_handle: LoopHandle<'static, CalloopData>,
    ) {
        let Backend::Udev(backend) = &mut self.backend else {
            unreachable!()
        };

        let device = if let Some(device) = backend.backends.get_mut(&node) {
            device
        } else {
            return;
        };

        let surface = if let Some(surface) = device.surfaces.get_mut(&crtc) {
            surface
        } else {
            return;
        };

        let node = surface.render_node;
        let result = {
            let mut renderer = backend.gpu_manager.single_renderer(&node).unwrap();
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

#[allow(clippy::too_many_arguments)]
fn render_surface<'a>(
    cursor_status: &mut CursorImageStatus,
    space: &Space<WindowElement>,
    windows: &[WindowElement],
    dnd_icon: Option<&WlSurface>,
    focus_stack: &[WindowElement],
    surface: &'a mut SurfaceData,
    renderer: &mut UdevRenderer<'a, '_>,
    output: &Output,
    input_method: &InputMethodHandle,
    pointer_image: &TextureBuffer<MultiTexture>,
    pointer_element: &mut PointerElement<MultiTexture>,
    pointer_location: Point<f64, Logical>,
    clock: &Clock<Monotonic>,
) -> Result<bool, SwapBuffersError> {
    let pending_wins = windows
        .iter()
        .filter(|win| win.alive())
        .filter(|win| win.with_state(|state| !state.loc_request_state.is_idle()))
        .map(|win| {
            (
                win.class().unwrap_or("None".to_string()),
                win.title().unwrap_or("None".to_string()),
                win.with_state(|state| state.loc_request_state.clone()),
            )
        })
        .collect::<Vec<_>>();

    if !pending_wins.is_empty() {
        // tracing::debug!("Skipping frame, waiting on {pending_wins:?}");
        for win in windows.iter() {
            win.send_frame(output, clock.now(), Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }

        surface
            .compositor
            .queue_frame(None, None, None)
            .map_err(Into::<SwapBuffersError>::into)?;

        // TODO: still draw the cursor here

        return Ok(true);
    }

    let output_render_elements = crate::render::generate_render_elements(
        space,
        focus_stack,
        pointer_location,
        cursor_status,
        dnd_icon,
        renderer,
        output,
        input_method,
        pointer_element,
        Some(pointer_image),
    );

    let res = surface.compositor.render_frame::<_, _, GlesTexture>(
        renderer,
        &output_render_elements,
        [0.6, 0.6, 0.6, 1.0],
    )?;

    let time = clock.now();

    // We need to send frames to the cursor surface so that xwayland windows will properly
    // update the cursor on motion.
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
            .queue_frame(res.sync, res.damage, Some(output_presentation_feedback))
            .map_err(Into::<SwapBuffersError>::into)?;
    }

    Ok(res.rendered)
}

fn initial_render(
    surface: &mut SurfaceData,
    renderer: &mut UdevRenderer<'_, '_>,
) -> Result<(), SwapBuffersError> {
    surface
        .compositor
        .render_frame::<_, CustomRenderElements<_>, GlesTexture>(
            renderer,
            &[],
            [0.6, 0.6, 0.6, 1.0],
        )?;
    surface.compositor.queue_frame(None, None, None)?;
    surface.compositor.reset_buffers();

    Ok(())
}
