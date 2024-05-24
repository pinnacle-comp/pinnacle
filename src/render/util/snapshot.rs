//! Utilities for capturing snapshots of windows and other elements.

use std::cell::OnceCell;
use std::rc::Rc;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::AsRenderElements;
use smithay::output::Output;
use smithay::{
    backend::renderer::{
        element::RenderElement,
        gles::{GlesRenderer, GlesTexture},
    },
    utils::{Physical, Point, Scale, Transform},
};
use tracing::error;

use crate::layout::transaction::LayoutSnapshot;
use crate::state::{Pinnacle, WithState};
use crate::window::WindowElement;

use super::{render_to_encompassing_texture, EncompassingTexture};

/// A snapshot of given elements that can be rendered at some point in the future.
#[derive(Debug)]
pub struct RenderSnapshot<E> {
    /// Rendered elements.
    ///
    /// These are not used directly in rendering due to floating-point rounding
    /// inaccuracies that cause pixel imperfections when being displayed.
    elements: Rc<Vec<E>>,
    /// The original scale used to create this snapshot.
    scale: Scale<f64>,
    /// The texture that elements will be rendered into.
    ///
    /// Happens lazily for performance.
    texture: OnceCell<(GlesTexture, Point<i32, Physical>)>,
}

impl<E> Clone for RenderSnapshot<E> {
    fn clone(&self) -> Self {
        Self {
            elements: self.elements.clone(),
            scale: self.scale,
            texture: self.texture.clone(),
        }
    }
}

impl<E: RenderElement<GlesRenderer>> RenderSnapshot<E> {
    /// Creates a new snapshot from elements.
    pub fn new(elements: impl IntoIterator<Item = E>, scale: Scale<f64>) -> Self {
        Self {
            elements: Rc::new(elements.into_iter().collect()),
            scale,
            texture: OnceCell::new(),
        }
    }

    /// Get the texture, rendering it to a new one if it doesn't exist.
    pub fn texture(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Option<(GlesTexture, Point<i32, Physical>)> {
        // Not `get_or_init` because that would require the cell be an option/result
        // and I didn't want that
        if self.texture.get().is_none() {
            let EncompassingTexture {
                texture,
                sync_point: _,
                loc,
            } = match render_to_encompassing_texture(
                renderer,
                self.elements.as_ref(),
                self.scale,
                Transform::Normal, // TODO: transform
                Fourcc::Argb8888,
            ) {
                Ok(tex) => tex,
                Err(err) => {
                    error!("Failed to render to encompassing texture: {err}");
                    return None;
                }
            };
            let Ok(()) = self.texture.set((texture, loc)) else {
                unreachable!()
            };
        }
        self.texture.get().cloned()
    }
}

impl WindowElement {
    /// Capture a snapshot for this window and store it in its user data.
    pub fn capture_snapshot_and_store(
        &self,
        renderer: &mut GlesRenderer,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) {
        let elements = self.render_elements(renderer, location, scale, alpha);
        self.with_state_mut(|state| {
            if state.snapshot.is_none() {
                tracing::info!("storing snapshot");
                state.snapshot = Some(RenderSnapshot::new(elements, scale));
            }
        })
    }
}

/// Capture snapshots for all tiled, fullscreen, and maximized windows on this output.
pub fn capture_snapshots_on_output(
    pinnacle: &mut Pinnacle,
    renderer: &mut GlesRenderer,
    output: &Output,
) -> impl Iterator<Item = LayoutSnapshot> {
    let windows_on_foc_tags = output.with_state(|state| {
        let focused_tags = state.focused_tags().collect::<Vec<_>>();
        pinnacle
            .windows
            .iter()
            .rev()
            .filter(|win| !win.is_x11_override_redirect())
            .filter(|win| {
                win.with_state(|state| state.tags.iter().any(|tg| focused_tags.contains(&tg)))
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let snapshot_windows = windows_on_foc_tags
        .iter()
        .filter(|win| {
            win.with_state(|state| {
                state.floating_or_tiled.is_tiled() || !state.fullscreen_or_maximized.is_neither()
            })
        })
        .cloned();

    let scale = output.current_scale().fractional_scale();

    let from_windows = snapshot_windows
        .filter_map(|win| {
            let loc: Point<i32, Physical> = (pinnacle.space.element_location(&win)?
                - win.geometry().loc
                - output.current_location())
            .to_physical_precise_round(scale);
            Some((win.clone(), loc))
        })
        .collect::<Vec<_>>();

    for (win, loc) in from_windows.iter() {
        win.capture_snapshot_and_store(
            renderer,
            *loc,
            output.current_scale().fractional_scale().into(),
            1.0,
        );
    }

    from_windows
        .into_iter()
        .flat_map(|(win, _)| win.with_state(|state| state.snapshot.clone()))
}
