use std::path::PathBuf;

use snowcap_api_defs::snowcap::widget;

use super::Length;

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub handle: Handle,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub expand: Option<bool>,
    pub content_fit: Option<ContentFit>,
    pub filter: Option<Filter>,
    /// Rotation in degrees.
    pub rotation: Option<f32>,
    pub opacity: Option<f32>,
    pub scale: Option<f32>,
}

impl Image {
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            width: None,
            height: None,
            expand: None,
            content_fit: None,
            filter: None,
            rotation: None,
            opacity: None,
            scale: None,
        }
    }

    pub fn width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn expand(self, expand: bool) -> Self {
        Self {
            expand: Some(expand),
            ..self
        }
    }

    pub fn content_fit(self, content_fit: ContentFit) -> Self {
        Self {
            content_fit: Some(content_fit),
            ..self
        }
    }

    pub fn filter(self, filter: Filter) -> Self {
        Self {
            filter: Some(filter),
            ..self
        }
    }

    pub fn rotation(self, degrees: f32) -> Self {
        Self {
            rotation: Some(degrees),
            ..self
        }
    }

    pub fn opacity(self, opacity: f32) -> Self {
        Self {
            opacity: Some(opacity),
            ..self
        }
    }

    pub fn scale(self, scale: f32) -> Self {
        Self {
            scale: Some(scale),
            ..self
        }
    }
}

impl From<Image> for widget::v1::Image {
    fn from(value: Image) -> Self {
        Self {
            width: value.width.map(From::from),
            height: value.height.map(From::from),
            expand: value.expand,
            content_fit: value
                .content_fit
                .map(|c| widget::v1::image::ContentFit::from(c) as i32),
            nearest_neighbor: value.filter.map(|filter| matches!(filter, Filter::Nearest)),
            rotation_degrees: value.rotation,
            opacity: value.opacity,
            scale: value.scale,
            handle: Some(match value.handle {
                Handle::Path(path_buf) => {
                    widget::v1::image::Handle::Path(path_buf.to_string_lossy().to_string())
                }
                Handle::Bytes(bytes) => widget::v1::image::Handle::Bytes(bytes),
                Handle::Rgba {
                    width,
                    height,
                    bytes,
                } => widget::v1::image::Handle::Rgba(widget::v1::image::Rgba {
                    width,
                    height,
                    rgba: bytes,
                }),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Handle {
    Path(PathBuf),
    Bytes(Vec<u8>),
    Rgba {
        width: u32,
        height: u32,
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ContentFit {
    Contain,
    Cover,
    Fill,
    None,
    ScaleDown,
}

impl From<ContentFit> for widget::v1::image::ContentFit {
    fn from(value: ContentFit) -> Self {
        match value {
            ContentFit::Contain => widget::v1::image::ContentFit::Contain,
            ContentFit::Cover => widget::v1::image::ContentFit::Cover,
            ContentFit::Fill => widget::v1::image::ContentFit::Fill,
            ContentFit::None => widget::v1::image::ContentFit::None,
            ContentFit::ScaleDown => widget::v1::image::ContentFit::ScaleDown,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Filter {
    Linear,
    Nearest,
}
