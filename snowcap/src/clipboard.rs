/// A no-op clipboard.
pub struct WaylandClipboard;

impl iced_wgpu::core::Clipboard for WaylandClipboard {
    fn read(&self, _kind: iced_wgpu::core::clipboard::Kind) -> Option<String> {
        None
    }

    fn write(&mut self, _kind: iced_wgpu::core::clipboard::Kind, _contents: String) {}
}
