use std::sync::Arc;

use anyhow::Context;
use iced_wgpu::graphics::backend::Text;
use iced_wgpu::{
    wgpu::{self, Backends},
    Backend,
};

use crate::block_on_tokio;

const UBUNTU_REGULAR: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Regular.ttf");
const UBUNTU_BOLD: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Bold.ttf");
const UBUNTU_ITALIC: &[u8] = include_bytes!("../resources/fonts/Ubuntu-Italic.ttf");
const UBUNTU_BOLD_ITALIC: &[u8] = include_bytes!("../resources/fonts/Ubuntu-BoldItalic.ttf");

pub struct Wgpu {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub renderer: iced_wgpu::Renderer,
}

pub fn setup_wgpu() -> anyhow::Result<Wgpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = block_on_tokio(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .context("no adapter")?;

    let (device, queue) = block_on_tokio(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(), // TODO:
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        },
        None,
    ))?;

    let mut backend = Backend::new(
        &device,
        &queue,
        iced_wgpu::Settings {
            present_mode: wgpu::PresentMode::Mailbox,
            internal_backend: Backends::VULKAN,
            ..Default::default()
        },
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );

    backend.load_font(UBUNTU_REGULAR.into());
    backend.load_font(UBUNTU_BOLD.into());
    backend.load_font(UBUNTU_ITALIC.into());
    backend.load_font(UBUNTU_BOLD_ITALIC.into());

    let renderer = iced_wgpu::Renderer::new(backend, Default::default(), iced::Pixels(16.0));

    Ok(Wgpu {
        instance: Arc::new(instance),
        adapter: Arc::new(adapter),
        device: Arc::new(device),
        queue: Arc::new(queue),
        renderer,
    })
}
