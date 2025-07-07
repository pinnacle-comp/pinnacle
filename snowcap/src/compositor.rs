pub type Compositor = iced_renderer::fallback::Compositor<
    crate::wgpu::Compositor,
    iced_tiny_skia::window::Compositor,
>;
pub type Renderer =
    iced_renderer::fallback::Renderer<iced_wgpu::Renderer, iced_tiny_skia::Renderer>;
