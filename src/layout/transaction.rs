//! Layout transactions.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use smithay::{
    backend::renderer::element::{
        self,
        surface::WaylandSurfaceRenderElement,
        texture::{TextureBuffer, TextureRenderElement},
        utils::RescaleRenderElement,
    },
    desktop::Space,
    reexports::calloop::{
        timer::{TimeoutAction, Timer},
        LoopHandle,
    },
    utils::{Logical, Point, Scale, Serial, Transform},
};

use crate::{
    pinnacle_render_elements,
    render::{
        texture::CommonTextureRenderElement, util::snapshot::RenderSnapshot, AsGlesRenderer,
        PRenderer,
    },
    state::State,
    window::WindowElement,
};

/// The timeout before transactions stop applying.
const TIMEOUT: Duration = Duration::from_millis(150);

/// Type for window snapshots.
pub type LayoutSnapshot = RenderSnapshot<CommonTextureRenderElement>;

pinnacle_render_elements! {
    /// Render elements for an output snapshot
    #[derive(Debug)]
    pub enum SnapshotRenderElement<R> {
        /// Draw the window itself.
        Window = WaylandSurfaceRenderElement<R>,
        /// Draw a snapshot of the window.
        Snapshot = RescaleRenderElement<CommonTextureRenderElement>,
    }
}

/// Specifier for snapshots
#[derive(Debug)]
pub enum SnapshotTarget {
    /// Render a window.
    Window(WindowElement),
    /// Render a snapshot.
    Snapshot(LayoutSnapshot),
}

/// A layout transaction.
///
/// While one is active on an output, its snapshots will be drawn instead of windows.
#[derive(Debug)]
pub struct LayoutTransaction {
    /// The loop handle to schedule event loop wakeups.
    loop_handle: LoopHandle<'static, State>,
    /// The instant this transaction started.
    ///
    /// Used for transaction timeout.
    start_time: Instant,
    /// The snapshots to render while the transaction is processing.
    pub fullscreen_and_up_snapshots: Vec<SnapshotTarget>,
    /// The snapshots to render while the transaction is processing.
    pub under_fullscreen_snapshots: Vec<SnapshotTarget>,
    /// The windows that the transaction is waiting on.
    pending_windows: HashMap<WindowElement, Serial>,
    /// Wait for an update to the windows this transaction is waiting on
    /// in anticipation of a new layout.
    wait: bool,
}

impl LayoutTransaction {
    /// Schedule an event after the timeout to check for readiness.
    fn register_wakeup(loop_handle: &LoopHandle<'static, State>) {
        let _ = loop_handle.insert_source(
            Timer::from_duration(TIMEOUT + Duration::from_millis(10)),
            |_, _, _| TimeoutAction::Drop,
        );
    }

    /// Creates a new layout transaction that will become immediately active.
    pub fn new(
        loop_handle: LoopHandle<'static, State>,
        fullscreen_and_up_snapshots: impl IntoIterator<Item = SnapshotTarget>,
        under_fullscreen_snapshots: impl IntoIterator<Item = SnapshotTarget>,
        pending_windows: impl IntoIterator<Item = (WindowElement, Serial)>,
    ) -> Self {
        Self::register_wakeup(&loop_handle);
        Self {
            loop_handle,
            start_time: Instant::now(),
            fullscreen_and_up_snapshots: fullscreen_and_up_snapshots.into_iter().collect(),
            under_fullscreen_snapshots: under_fullscreen_snapshots.into_iter().collect(),
            pending_windows: pending_windows.into_iter().collect(),
            wait: false,
        }
    }

    /// Wait for the next pending window update.
    pub fn wait(&mut self) {
        self.wait = true;
        self.start_time = Instant::now();
        Self::register_wakeup(&self.loop_handle);
    }

    /// Creates a new layout transaction that waits for the next update to pending windows.
    pub fn new_and_wait(
        loop_handle: LoopHandle<'static, State>,
        fullscreen_and_up_snapshots: impl IntoIterator<Item = SnapshotTarget>,
        under_fullscreen_snapshots: impl IntoIterator<Item = SnapshotTarget>,
    ) -> Self {
        Self::register_wakeup(&loop_handle);
        Self {
            loop_handle,
            start_time: Instant::now(),
            fullscreen_and_up_snapshots: fullscreen_and_up_snapshots.into_iter().collect(),
            under_fullscreen_snapshots: under_fullscreen_snapshots.into_iter().collect(),
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
        Self::register_wakeup(&self.loop_handle);
    }

    /// Returns whether all pending windows have committed their serials or the timeout has been
    /// reached.
    pub fn ready(&self) -> bool {
        self.start_time.elapsed() >= TIMEOUT
            || (!self.wait
                && self
                    .pending_windows
                    .iter()
                    .all(|(win, serial)| win.is_serial_committed(*serial)))
    }

    /// Render elements for this transaction, split into ones for windows fullscreen and up
    /// and the rest.
    ///
    /// Window targets will be rendered normally and snapshot targets will
    /// render their texture.
    pub fn render_elements<R: PRenderer + AsGlesRenderer>(
        &self,
        renderer: &mut R,
        space: &Space<WindowElement>,
        output_loc: Point<i32, Logical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> (Vec<SnapshotRenderElement<R>>, Vec<SnapshotRenderElement<R>>) {
        let mut flat_map = |snapshot: &SnapshotTarget| match snapshot {
            SnapshotTarget::Window(window) => {
                let loc = space.element_location(window).unwrap_or_default() - output_loc;
                window
                    .render_elements(renderer, loc, scale, alpha)
                    .into_iter()
                    .map(SnapshotRenderElement::Window)
                    .collect()
            }
            SnapshotTarget::Snapshot(snapshot) => {
                let Some((texture, loc)) = snapshot.texture(renderer.as_gles_renderer()) else {
                    return Vec::new();
                };
                let buffer =
                    TextureBuffer::from_texture(renderer, texture, 1, Transform::Normal, None);
                let elem = TextureRenderElement::from_texture_buffer(
                    loc.to_f64(),
                    &buffer,
                    Some(alpha),
                    None,
                    None,
                    element::Kind::Unspecified,
                );

                let common = CommonTextureRenderElement::new(elem);

                let scale = Scale::from((1.0 / scale.x, 1.0 / scale.y));

                vec![SnapshotRenderElement::Snapshot(
                    RescaleRenderElement::from_element(common, loc, scale),
                )]
            }
        };

        (
            self.fullscreen_and_up_snapshots
                .iter()
                .flat_map(&mut flat_map)
                .collect(),
            self.under_fullscreen_snapshots
                .iter()
                .flat_map(&mut flat_map)
                .collect(),
        )
    }
}
