// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;
use std::{collections::HashMap, rc::Rc};

use anyhow::Context;
use smithay::backend::allocator::Fourcc;
use smithay::utils::IsAlive;
use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    input::pointer::{CursorIcon, CursorImageStatus},
    utils::Transform,
};
use xcursor::{
    CursorTheme,
    parser::{Image, parse_xcursor},
};

use crate::render::pointer::PointerElement;

static FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../resources/cursor.rgba");

pub struct CursorState {
    current_cursor_image: CursorImageStatus,
    theme: CursorTheme,
    size: u32,
    mem_buffer_cache: Vec<(Image, MemoryRenderBuffer)>,
    /// A map of cursor icons to loaded images
    loaded_images: HashMap<CursorIcon, Option<Rc<XCursor>>>,
}

impl CursorState {
    pub fn new() -> Self {
        let (theme, size) = load_xcursor_theme_from_env();

        // SAFETY: All set_vars occur on the event loop thread
        unsafe {
            std::env::set_var("XCURSOR_THEME", &theme);
            std::env::set_var("XCURSOR_SIZE", size.to_string());
        }

        Self {
            current_cursor_image: CursorImageStatus::default_named(),
            theme: CursorTheme::load(&theme),
            size,
            mem_buffer_cache: Default::default(),
            loaded_images: Default::default(),
        }
    }

    pub fn set_theme(&mut self, theme: &str) {
        // SAFETY: All set_vars occur on the event loop thread
        unsafe {
            std::env::set_var("XCURSOR_THEME", theme);
        }

        self.theme = CursorTheme::load(theme);
        self.mem_buffer_cache.clear();
        self.loaded_images.clear();
    }

    pub fn set_size(&mut self, size: u32) {
        // SAFETY: All set_vars occur on the event loop thread
        unsafe {
            std::env::set_var("XCURSOR_SIZE", size.to_string());
        }

        self.size = size;
        self.mem_buffer_cache.clear();
        self.loaded_images.clear();
    }

    pub fn cursor_size(&self, scale: i32) -> u32 {
        self.size * scale as u32
    }

    pub fn set_cursor_image(&mut self, image: CursorImageStatus) {
        self.current_cursor_image = image;
    }

    pub fn cursor_image(&self) -> &CursorImageStatus {
        &self.current_cursor_image
    }

    pub fn get_xcursor_images(&mut self, icon: CursorIcon) -> Option<Rc<XCursor>> {
        let _span = tracy_client::span!("CursorState::get_xcursor_images");

        self.loaded_images
            .entry(icon)
            .or_insert_with_key(|icon| {
                let mut images = load_xcursor_images(&self.theme, *icon);
                if *icon == CursorIcon::Default && images.is_err() {
                    images = Ok(fallback_cursor());
                }
                images.ok().map(Rc::new)
            })
            .clone()
    }

    pub fn buffer_for_image(&mut self, image: Image, scale: i32) -> MemoryRenderBuffer {
        let _span = tracy_client::span!("CursorState::buffer_for_image");

        self.mem_buffer_cache
            .iter()
            .find_map(|(img, buf)| (*img == image).then(|| buf.clone()))
            .unwrap_or_else(|| {
                // TODO: scale
                let buffer = MemoryRenderBuffer::from_slice(
                    &image.pixels_rgba,
                    // Don't make Abgr, then the format doesn't match the
                    // cursor bo and this doesn't get put on the cursor plane
                    Fourcc::Argb8888,
                    (image.width as i32, image.height as i32),
                    scale,
                    Transform::Normal,
                    None,
                );

                self.mem_buffer_cache.push((image, buffer.clone()));

                buffer
            })
    }

    pub fn pointer_element(&mut self) -> PointerElement {
        let _span = tracy_client::span!("CursorState::pointer_element");

        match &self.current_cursor_image {
            CursorImageStatus::Hidden => PointerElement::Hidden,
            CursorImageStatus::Named(icon) => {
                let cursor = self
                    .get_xcursor_images(*icon)
                    .or_else(|| self.get_xcursor_images(CursorIcon::Default))
                    .unwrap();
                PointerElement::Named {
                    cursor,
                    size: self.size,
                }
            }
            CursorImageStatus::Surface(surface) => PointerElement::Surface {
                surface: surface.clone(),
            },
        }
    }

    pub fn is_current_cursor_animated(&mut self) -> bool {
        let _span = tracy_client::span!("CursorState::is_current_cursor_animated");

        match &self.current_cursor_image {
            CursorImageStatus::Hidden => false,
            CursorImageStatus::Named(icon) => {
                let cursor = self
                    .get_xcursor_images(*icon)
                    .or_else(|| self.get_xcursor_images(CursorIcon::Default))
                    .unwrap();

                let img_count = nearest_size_images(self.size, &cursor.images).count();

                let is_animated = img_count > 1;
                is_animated
            }
            CursorImageStatus::Surface(_) => false,
        }
    }

    /// Cleans up the current cursor if it is a dead WlSurface.
    pub fn cleanup(&mut self) {
        let _span = tracy_client::span!("CursorState::cleanup");

        if let CursorImageStatus::Surface(surface) = &self.current_cursor_image
            && !surface.alive()
        {
            self.current_cursor_image = CursorImageStatus::default_named();
        }
    }
}

pub struct XCursor {
    images: Vec<Image>,
}

impl XCursor {
    pub fn image(&self, time: Duration, size: u32) -> Image {
        let _span = tracy_client::span!("XCursor::image");

        let mut millis = time.as_millis() as u32;

        let animation_length_ms =
            nearest_size_images(size, &self.images).fold(0, |acc, image| acc + image.delay);

        millis = millis.checked_rem(animation_length_ms).unwrap_or_default();

        for img in nearest_size_images(size, &self.images) {
            if millis <= img.delay {
                return img.clone();
            }
            millis -= img.delay;
        }

        unreachable!()
    }
}

fn nearest_size_images(size: u32, images: &[Image]) -> impl Iterator<Item = &Image> {
    let _span = tracy_client::span!("crate::cursor::nearest_size_images");

    // Follow the nominal size of the cursor to choose the nearest
    let nearest_image = images
        .iter()
        .min_by_key(|image| (size as i32 - image.size as i32).abs())
        .unwrap();

    images.iter().filter(move |image| {
        image.width == nearest_image.width && image.height == nearest_image.height
    })
}

/// Loads a theme and size from $XCURSOR_THEME and $XCURSOR_SIZE.
///
/// Defaults to "default" and 24 respectively.
fn load_xcursor_theme_from_env() -> (String, u32) {
    let theme = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".into());
    let size = std::env::var("XCURSOR_SIZE")
        .ok()
        .and_then(|size| size.parse::<u32>().ok())
        .unwrap_or(24);

    (theme, size)
}

/// Load xcursor images for the given theme and icon.
///
/// Looks through legacy names as fallback.
fn load_xcursor_images(theme: &CursorTheme, icon: CursorIcon) -> anyhow::Result<XCursor> {
    let icon_path = std::iter::once(&icon.name())
        .chain(icon.alt_names())
        .find_map(|name| theme.load_icon(name))
        .context("no images for icon")?;

    let cursor_bytes = std::fs::read(icon_path).context("failed to read xcursor file")?;

    parse_xcursor(&cursor_bytes)
        .map(|images| XCursor { images })
        .context("failed to parse xcursor bytes")
}

fn fallback_cursor() -> XCursor {
    XCursor {
        images: vec![Image {
            size: 32,
            width: 64,
            height: 64,
            xhot: 1,
            yhot: 1,
            delay: 1,
            pixels_rgba: Vec::from(FALLBACK_CURSOR_DATA),
            pixels_argb: vec![], // unused
        }],
    }
}
