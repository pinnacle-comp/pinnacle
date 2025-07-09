use anyhow::Context;
use iced_graphics::Compositor as _;
use iced_wgpu::wgpu;

use crate::util::BlockOnTokio;

const UBUNTU_REGULAR: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Regular.ttf");
const UBUNTU_BOLD: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Bold.ttf");
const UBUNTU_ITALIC: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Italic.ttf");
const UBUNTU_BOLD_ITALIC: &[u8] = include_bytes!("../resources/fonts/Ubuntu-BoldItalic.ttf");

pub struct Compositor {
    instance: wgpu::Instance,
    device: wgpu::Device,
    adapter: wgpu::Adapter,
    _format: wgpu::TextureFormat,
    _alpha_mode: wgpu::CompositeAlphaMode,
    engine: iced_wgpu::Engine,
}

impl Compositor {
    pub fn new() -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::default().with_env(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .block_on_tokio()
            .context("no adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .block_on_tokio()?;

        let engine = iced_wgpu::Engine::new(
            &adapter,
            device.clone(),
            queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None, // TODO:
        );

        let mut compositor = Compositor {
            instance,
            device,
            adapter,
            _format: wgpu::TextureFormat::Rgba8UnormSrgb,
            _alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
            engine,
        };

        compositor.load_font(UBUNTU_REGULAR.into());
        compositor.load_font(UBUNTU_BOLD.into());
        compositor.load_font(UBUNTU_ITALIC.into());
        compositor.load_font(UBUNTU_BOLD_ITALIC.into());

        Ok(compositor)
    }
}

impl iced_graphics::Compositor for Compositor {
    type Renderer = iced_wgpu::Renderer;

    type Surface = wgpu::Surface<'static>;

    async fn with_backend<W: iced_graphics::compositor::Window + Clone>(
        _settings: iced_graphics::Settings,
        _compatible_window: W,
        backend: Option<&str>,
    ) -> Result<Self, iced_graphics::Error> {
        match backend {
            None | Some("wgpu") => Ok(Compositor::new().map_err(|err| {
                iced_graphics::Error::GraphicsAdapterNotFound {
                    backend: "wgpu",
                    reason: iced_graphics::error::Reason::RequestFailed(err.to_string()),
                }
            })?),
            Some(backend) => Err(iced_graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: iced_graphics::error::Reason::DidNotMatch {
                    preferred_backend: backend.to_string(),
                },
            }),
        }
    }

    fn create_renderer(&self) -> Self::Renderer {
        iced_wgpu::Renderer::new(self.engine.clone(), Default::default(), iced::Pixels(16.0))
    }

    fn create_surface<W: iced_graphics::compositor::Window + Clone>(
        &mut self,
        window: W,
        width: u32,
        height: u32,
    ) -> Self::Surface {
        let mut surface = self.instance.create_surface(window).unwrap();

        if width > 0 && height > 0 {
            self.configure_surface(&mut surface, width, height);
        }

        surface
    }

    fn configure_surface(&mut self, surface: &mut Self::Surface, width: u32, height: u32) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            present_mode: iced_wgpu::wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 1,
            alpha_mode: iced_wgpu::wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![iced_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb],
        };

        surface.configure(&self.device, &surface_config);
    }

    fn fetch_information(&self) -> iced_graphics::compositor::Information {
        let information = self.adapter.get_info();

        iced_graphics::compositor::Information {
            adapter: information.name,
            backend: format!("{:?}", information.backend),
        }
    }

    fn present(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &iced_graphics::Viewport,
        background_color: iced::Color,
        on_pre_present: impl FnOnce(),
    ) -> Result<(), iced_graphics::compositor::SurfaceError> {
        iced_wgpu::window::compositor::present(
            renderer,
            surface,
            viewport,
            background_color,
            on_pre_present,
        )
    }

    fn screenshot(
        &mut self,
        renderer: &mut Self::Renderer,
        viewport: &iced_graphics::Viewport,
        background_color: iced::Color,
    ) -> Vec<u8> {
        renderer.screenshot(viewport, background_color)
    }
}
