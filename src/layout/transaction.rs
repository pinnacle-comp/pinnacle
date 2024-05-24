#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Layout transactions.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use smithay::{
    backend::renderer::{
        element::{
            self,
            surface::WaylandSurfaceRenderElement,
            texture::{TextureBuffer, TextureRenderElement},
            utils::RescaleRenderElement,
            AsRenderElements,
        },
        gles::GlesRenderer,
    },
    utils::{Physical, Point, Scale, Serial, Transform},
};

use crate::{
    render::{texture::CommonTextureRenderElement, util::snapshot::RenderSnapshot},
    window::WindowElement,
};

/// Type for window snapshots.
pub type LayoutSnapshot = RenderSnapshot<WaylandSurfaceRenderElement<GlesRenderer>>;

/// A layout transaction.
///
/// While one is active on an output, its snapshots will be drawn instead of windows.
#[derive(Debug)]
pub struct LayoutTransaction {
    /// The instant this transaction started.
    ///
    /// Used for transaction timeout.
    start_time: Instant,
    /// The snapshots to render while the transaction is processing.
    snapshots: Vec<LayoutSnapshot>,
    /// The windows that the transaction is waiting on.
    pending_windows: HashMap<WindowElement, Serial>,
    /// Wait for an update to the windows this transaction is waiting on
    /// in anticipation of a new layout.
    wait: bool,
}

impl LayoutTransaction {
    /// Creates a new layout transaction that will become immediately active.
    pub fn new(
        snapshots: impl IntoIterator<Item = LayoutSnapshot>,
        pending_windows: impl IntoIterator<Item = (WindowElement, Serial)>,
    ) -> Self {
        Self {
            start_time: Instant::now(),
            snapshots: snapshots.into_iter().collect(),
            pending_windows: pending_windows.into_iter().collect(),
            wait: false,
        }
    }

    /// Wait for the next pending window update.
    pub fn wait(&mut self) {
        self.wait = true;
        self.start_time = Instant::now();
    }

    /// Creates a new layout transaction that waits for the next update to pending windows.
    pub fn new_and_wait(snapshots: impl IntoIterator<Item = LayoutSnapshot>) -> Self {
        Self {
            start_time: Instant::now(),
            snapshots: snapshots.into_iter().collect(),
            pending_windows: HashMap::new(),
            wait: true,
        }
    }

    /// Updates the pending windows for this transaction, for example
    /// when a new layout comes in while a transaction is already processing.
    pub fn update_pending(
        &mut self,
        pending_windows: impl IntoIterator<Item = (WindowElement, Serial)>,
    ) {
        self.pending_windows = pending_windows.into_iter().collect();
        self.wait = false;
        self.start_time = Instant::now();
    }

    /// Returns whether all pending windows have committed their serials or the timeout has been
    /// reached.
    pub fn ready(&self) -> bool {
        Instant::now().duration_since(self.start_time) >= Duration::from_millis(1000)
        // || (!self.wait
        //     && self
        //         .pending_windows
        //         .iter()
        //         .all(|(win, serial)| win.is_serial_committed(*serial)))
    }
}

impl AsRenderElements<GlesRenderer> for LayoutTransaction {
    type RenderElement = RescaleRenderElement<CommonTextureRenderElement>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut GlesRenderer,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        self.snapshots
            .iter()
            .flat_map(|snapshot| {
                let (texture, loc) = snapshot.texture(renderer)?;
                let buffer =
                    TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
                let elem = TextureRenderElement::from_texture_buffer(
                    (loc + location).to_f64(),
                    &buffer,
                    Some(alpha),
                    None,
                    None,
                    element::Kind::Unspecified,
                );

                let common = CommonTextureRenderElement::new(elem);

                let scale = Scale::from((1.0 / scale.x, 1.0 / scale.y));

                Some(C::from(RescaleRenderElement::from_element(
                    common, loc, scale,
                )))
            })
            .collect()
    }
}
