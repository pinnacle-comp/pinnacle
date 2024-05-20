use std::collections::HashMap;

use smithay::{
    backend::renderer::gles::GlesTexture,
    utils::{Logical, Point, Serial},
};

use crate::window::WindowElement;

pub struct LayoutTransaction {
    from: Vec<(GlesTexture)>,
    to: HashMap<WindowElement, PendingLayoutState>,
}

pub struct PendingLayoutState {
    serial: Serial,
    loc: Point<i32, Logical>,
}
