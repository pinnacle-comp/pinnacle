use std::ffi::c_void;

/// A newtype wrapper over [`smithay_clipboard::Clipboard`].
pub struct WaylandClipboard(smithay_clipboard::Clipboard);

impl WaylandClipboard {
    /// # Safety
    ///
    /// `display` must be a valid `*mut wl_display` pointer, and it must remain
    /// valid for as long as `Clipboard` object is alive.
    pub unsafe fn new(display: *mut c_void) -> Self {
        Self(smithay_clipboard::Clipboard::new(display))
    }
}

impl iced_wgpu::core::Clipboard for WaylandClipboard {
    fn read(&self, _kind: iced_wgpu::core::clipboard::Kind) -> Option<String> {
        self.0.load().ok()
    }

    fn write(&mut self, _kind: iced_wgpu::core::clipboard::Kind, contents: String) {
        self.0.store(contents)
    }
}
