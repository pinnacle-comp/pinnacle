// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Layout management.
//!
//! TODO: finish this documentation

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use pinnacle_api_defs::pinnacle::layout::v0alpha1::{
    layout_request::{Body, ExplicitLayout, Geometries},
    layout_service_client::LayoutServiceClient,
    LayoutRequest,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::StreamExt;
use tonic::transport::Channel;

use crate::{
    block_on_tokio,
    output::OutputHandle,
    tag::TagHandle,
    util::{Axis, Geometry},
    window::WindowHandle,
    OUTPUT, TAG, WINDOW,
};

/// A struct that allows you to add and remove tags and get [`TagHandle`]s.
#[derive(Clone, Debug)]
pub struct Layout {
    layout_client: LayoutServiceClient<Channel>,
}

impl Layout {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            layout_client: LayoutServiceClient::new(channel.clone()),
        }
    }

    /// Consume the given [`LayoutManager`] and set it as the global layout handler.
    ///
    /// This returns a [`LayoutRequester`] that allows you to manually request layouts from
    /// the compositor. The requester also contains your layout manager wrapped in an `Arc<Mutex>`
    /// to allow you to mutate its settings.
    pub fn set_manager<M>(&self, manager: M) -> LayoutRequester<M>
    where
        M: LayoutManager + Send + 'static,
    {
        let (from_client, to_server) = unbounded_channel::<LayoutRequest>();
        let to_server_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(to_server);
        let mut from_server = block_on_tokio(self.layout_client.clone().layout(to_server_stream))
            .expect("TODO")
            .into_inner();

        let from_client_clone = from_client.clone();

        let manager = Arc::new(Mutex::new(manager));

        let requester = LayoutRequester {
            sender: from_client_clone,
            manager: manager.clone(),
        };

        let thing = async move {
            while let Some(Ok(response)) = from_server.next().await {
                let args = LayoutArgs {
                    output: OUTPUT.get().unwrap().new_handle(response.output_name()),
                    windows: response
                        .window_ids
                        .into_iter()
                        .map(|id| WINDOW.get().unwrap().new_handle(id))
                        .collect(),
                    tags: response
                        .tag_ids
                        .into_iter()
                        .map(|id| TAG.get().unwrap().new_handle(id))
                        .collect(),
                    output_width: response.output_width.unwrap_or_default(),
                    output_height: response.output_height.unwrap_or_default(),
                };
                let geos = manager.lock().unwrap().active_layout(&args).layout(&args);
                from_client
                    .send(LayoutRequest {
                        body: Some(Body::Geometries(Geometries {
                            request_id: response.request_id,
                            output_name: response.output_name,
                            geometries: geos
                                .into_iter()
                                .map(|geo| pinnacle_api_defs::pinnacle::v0alpha1::Geometry {
                                    x: Some(geo.x),
                                    y: Some(geo.y),
                                    width: Some(geo.width as i32),
                                    height: Some(geo.height as i32),
                                })
                                .collect(),
                        })),
                    })
                    .unwrap();
            }
        };

        tokio::spawn(thing);
        requester
    }
}

/// Arguments that [`LayoutGenerator`]s receive when a layout is requested.
#[derive(Clone, Debug)]
pub struct LayoutArgs {
    /// The output that is being laid out.
    pub output: OutputHandle,
    /// The windows that are being laid out.
    pub windows: Vec<WindowHandle>,
    /// The *focused* tags on the output.
    pub tags: Vec<TagHandle>,
    /// The width of the layout area, in pixels.
    pub output_width: u32,
    /// The height of the layout area, in pixels.
    pub output_height: u32,
}

/// Types that can manage layouts.
pub trait LayoutManager {
    /// Get the currently active layout for layouting.
    fn active_layout(&mut self, args: &LayoutArgs) -> &dyn LayoutGenerator;
}

/// Types that can generate layouts by computing a vector of [geometries][Geometry].
pub trait LayoutGenerator {
    /// Generate a vector of [geometries][Geometry] using the given [`LayoutArgs`].
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry>;
}

/// Gaps between windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gaps {
    /// An absolute amount of pixels between windows and the edge of the output.
    ///
    /// For example, `Gaps::Absolute(8)` means there will be 8 pixels between each window
    /// and between the edge of the output.
    Absolute(u32),
    /// A split amount of pixels between windows and the edge of the output.
    Split {
        /// The amount of gap in pixels around *each* window.
        ///
        /// For example, `Gaps::Split { inner: 2, ... }` means there will be
        /// 4 pixels between windows, 2 around each window.
        inner: u32,
        /// The amount of gap in pixels inset from the edge of the output.
        outer: u32,
    },
}

/// A [`LayoutManager`] that keeps track of layouts per output and provides
/// methods to cycle between them.
pub struct CyclingLayoutManager {
    layouts: Vec<Box<dyn LayoutGenerator + Send>>,
    tag_indices: HashMap<u32, usize>,
}

impl CyclingLayoutManager {
    /// Create a new [`CyclingLayoutManager`] from the given [`LayoutGenerator`]s.
    ///
    /// `LayoutGenerator`s must be boxed then coerced to trait objects, so you
    /// will need to do an unsizing cast to use them here.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::layout::CyclingLayoutManager;
    /// use pinnacle_api::layout::{MasterStackLayout, DwindleLayout, CornerLayout};
    ///
    /// let cycling_layout_manager = CyclingLayoutManager::new([
    ///     // The `as _` is necessary to coerce to a boxed trait object
    ///     Box::<MasterStackLayout>::default() as _,
    ///     Box::<DwindleLayout>::default() as _,
    ///     Box::<CornerLayout>::default() as _,
    /// ]);
    /// ```
    pub fn new(layouts: impl IntoIterator<Item = Box<dyn LayoutGenerator + Send>>) -> Self {
        Self {
            layouts: layouts.into_iter().collect(),
            tag_indices: HashMap::default(),
        }
    }

    /// Cycle the layout forward on the given tag.
    pub fn cycle_layout_forward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        *index += 1;
        if *index >= self.layouts.len() {
            *index = 0;
        }
    }

    /// Cycle the layout backward on the given tag.
    pub fn cycle_layout_backward(&mut self, tag: &TagHandle) {
        let index = self.tag_indices.entry(tag.id).or_default();
        if let Some(i) = index.checked_sub(1) {
            *index = i;
        } else {
            *index = self.layouts.len().saturating_sub(1);
        }
    }
}

impl LayoutManager for CyclingLayoutManager {
    fn active_layout(&mut self, args: &LayoutArgs) -> &dyn LayoutGenerator {
        let Some(first_tag) = args.tags.first() else {
            return &NoopLayout;
        };

        self.layouts
            .get(*self.tag_indices.entry(first_tag.id).or_default())
            .expect("no layouts in manager")
            .as_ref()
    }
}

/// A struct that can request layouts and provides access to a consumed [`LayoutManager`].
#[derive(Debug)]
pub struct LayoutRequester<T> {
    sender: UnboundedSender<LayoutRequest>,
    /// The manager that was consumed, wrapped in an `Arc<Mutex>`.
    pub manager: Arc<Mutex<T>>,
}

impl<T> Clone for LayoutRequester<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl<T> LayoutRequester<T> {
    /// Request a layout from the compositor.
    ///
    /// This uses the focused output for the request.
    /// If you want to layout a specific output, see [`LayoutRequester::request_layout_on_output`].
    pub fn request_layout(&self) {
        let output_name = OUTPUT.get().unwrap().get_focused().map(|op| op.name);
        self.sender
            .send(LayoutRequest {
                body: Some(Body::Layout(ExplicitLayout { output_name })),
            })
            .unwrap();
    }

    /// Request a layout from the compositor for the given output.
    pub fn request_layout_on_output(&self, output: &OutputHandle) {
        self.sender
            .send(LayoutRequest {
                body: Some(Body::Layout(ExplicitLayout {
                    output_name: Some(output.name.clone()),
                })),
            })
            .unwrap();
    }
}

impl LayoutRequester<CyclingLayoutManager> {
    /// Cycle the layout forward for the given tag.
    pub fn cycle_layout_forward(&self, tag: &TagHandle) {
        let mut lock = self.manager.lock().unwrap();
        lock.cycle_layout_forward(tag);
    }

    /// Cycle the layout backward for the given tag.
    pub fn cycle_layout_backward(&mut self, tag: &TagHandle) {
        let mut lock = self.manager.lock().unwrap();
        lock.cycle_layout_backward(tag);
    }
}

/// A layout generator that does nothing.
struct NoopLayout;

impl LayoutGenerator for NoopLayout {
    fn layout(&self, _args: &LayoutArgs) -> Vec<Geometry> {
        Vec::new()
    }
}

/// Which side the master area will be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MasterSide {
    /// The master area will be on the left.
    Left,
    /// The master area will be on the right.
    Right,
    /// The master area will be at the top.
    Top,
    /// The master area will be at the bottom.
    Bottom,
}

/// A [`LayoutGenerator`] that has one master area to one side and a stack of windows
/// next to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MasterStackLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The proportion of the output the master area will take up.
    ///
    /// This will be clamped between 0.1 and 0.9.
    ///
    /// Defaults to 0.5
    pub master_factor: f32,
    /// Which side the master area will be.
    ///
    /// Defaults to [`MasterSide::Left`].
    pub master_side: MasterSide,
    /// How many windows will be in the master area.
    ///
    /// Defaults to 1.
    pub master_count: u32,
}

impl Default for MasterStackLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            master_factor: 0.5,
            master_side: MasterSide::Left,
            master_count: 1,
        }
    }
}

impl LayoutGenerator for MasterStackLayout {
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
        let win_count = args.windows.len() as u32;

        if win_count == 0 {
            return Vec::new();
        }

        let width = args.output_width;
        let height = args.output_height;

        let mut geos = Vec::<Geometry>::new();

        let (outer_gaps, inner_gaps) = match self.gaps {
            Gaps::Absolute(gaps) => (gaps, None),
            Gaps::Split { inner, outer } => (outer, Some(inner)),
        };

        let rect = Geometry {
            x: 0,
            y: 0,
            width,
            height,
        }
        .split_at(Axis::Horizontal, 0, outer_gaps)
        .0
        .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
        .0
        .split_at(Axis::Vertical, 0, outer_gaps)
        .0
        .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
        .0;

        let master_factor = if win_count > self.master_count {
            self.master_factor.clamp(0.1, 0.9)
        } else {
            1.0
        };

        let gaps = match inner_gaps {
            Some(_) => 0,
            None => outer_gaps,
        };

        let (master_rect, mut stack_rect) = match self.master_side {
            MasterSide::Left => {
                let (rect1, rect2) = rect.split_at(
                    Axis::Vertical,
                    (width as f32 * master_factor).floor() as i32 - gaps as i32 / 2,
                    gaps,
                );
                (Some(rect1), rect2)
            }
            MasterSide::Right => {
                let (rect2, rect1) = rect.split_at(
                    Axis::Vertical,
                    (width as f32 * master_factor).floor() as i32 - gaps as i32 / 2,
                    gaps,
                );
                (rect1, Some(rect2))
            }
            MasterSide::Top => {
                let (rect1, rect2) = rect.split_at(
                    Axis::Horizontal,
                    (height as f32 * master_factor).floor() as i32 - gaps as i32 / 2,
                    gaps,
                );
                (Some(rect1), rect2)
            }
            MasterSide::Bottom => {
                let (rect2, rect1) = rect.split_at(
                    Axis::Horizontal,
                    (height as f32 * master_factor).floor() as i32 - gaps as i32 / 2,
                    gaps,
                );
                (rect1, Some(rect2))
            }
        };

        let mut master_rect = master_rect.unwrap_or_else(|| stack_rect.take().unwrap());

        let (master_count, stack_count) = if win_count > self.master_count {
            (self.master_count, Some(win_count - self.master_count))
        } else {
            (win_count, None)
        };

        if master_count > 1 {
            let (coord, len, axis) = match self.master_side {
                MasterSide::Left | MasterSide::Right => (
                    master_rect.y,
                    master_rect.height as f32 / master_count as f32,
                    Axis::Horizontal,
                ),
                MasterSide::Top | MasterSide::Bottom => (
                    master_rect.x,
                    master_rect.width as f32 / master_count as f32,
                    Axis::Vertical,
                ),
            };

            for i in 1..master_count {
                let slice_point = coord + (len * i as f32) as i32 - gaps as i32 / 2;
                let (to_push, rest) = master_rect.split_at(axis, slice_point, gaps);
                geos.push(to_push);
                if let Some(rest) = rest {
                    master_rect = rest;
                } else {
                    break;
                }
            }
        }

        geos.push(master_rect);

        if let Some(stack_count) = stack_count {
            let mut stack_rect = stack_rect.unwrap();

            if stack_count > 1 {
                let (coord, len, axis) = match self.master_side {
                    MasterSide::Left | MasterSide::Right => (
                        stack_rect.y,
                        stack_rect.height as f32 / stack_count as f32,
                        Axis::Horizontal,
                    ),
                    MasterSide::Top | MasterSide::Bottom => (
                        stack_rect.x,
                        stack_rect.width as f32 / stack_count as f32,
                        Axis::Vertical,
                    ),
                };

                for i in 1..stack_count {
                    let slice_point = coord + (len * i as f32) as i32 - gaps as i32 / 2;
                    let (to_push, rest) = stack_rect.split_at(axis, slice_point, gaps);
                    geos.push(to_push);
                    if let Some(rest) = rest {
                        stack_rect = rest;
                    } else {
                        break;
                    }
                }
            }

            geos.push(stack_rect);
        }

        if let Some(inner_gaps) = inner_gaps {
            for geo in geos.iter_mut() {
                geo.x += inner_gaps as i32;
                geo.y += inner_gaps as i32;
                geo.width -= inner_gaps * 2;
                geo.height -= inner_gaps * 2;
            }
        }

        geos
    }
}

/// A [`LayoutGenerator`] that lays out windows in a shrinking fashion
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct DwindleLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The ratio for each dwindle split.
    ///
    /// The first split will use the factor at key `1`,
    /// the second at key `2`, and so on.
    ///
    /// Splits without a factor will default to 0.5.
    pub split_factors: HashMap<usize, f32>,
}

impl Default for DwindleLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            split_factors: Default::default(),
        }
    }
}

impl LayoutGenerator for DwindleLayout {
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
        let win_count = args.windows.len() as u32;

        if win_count == 0 {
            return Vec::new();
        }

        let width = args.output_width;
        let height = args.output_height;

        let mut geos = Vec::<Geometry>::new();

        let (outer_gaps, inner_gaps) = match self.gaps {
            Gaps::Absolute(gaps) => (gaps, None),
            Gaps::Split { inner, outer } => (outer, Some(inner)),
        };

        let gaps = match inner_gaps {
            Some(_) => 0,
            None => outer_gaps,
        };

        let mut rect = Geometry {
            x: 0,
            y: 0,
            width,
            height,
        }
        .split_at(Axis::Horizontal, 0, outer_gaps)
        .0
        .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
        .0
        .split_at(Axis::Vertical, 0, outer_gaps)
        .0
        .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
        .0;

        if win_count == 1 {
            geos.push(rect)
        } else {
            for i in 1..win_count {
                let factor = self
                    .split_factors
                    .get(&(i as usize))
                    .copied()
                    .unwrap_or(0.5)
                    .clamp(0.1, 0.9);

                let (axis, mut split_coord) = if i % 2 == 1 {
                    (Axis::Vertical, rect.x + (rect.width as f32 * factor) as i32)
                } else {
                    (
                        Axis::Horizontal,
                        rect.y + (rect.height as f32 * factor) as i32,
                    )
                };
                split_coord -= gaps as i32 / 2;

                let (to_push, rest) = rect.split_at(axis, split_coord, gaps);

                geos.push(to_push);

                if let Some(rest) = rest {
                    rect = rest;
                } else {
                    break;
                }
            }

            geos.push(rect)
        }

        if let Some(inner_gaps) = inner_gaps {
            for geo in geos.iter_mut() {
                geo.x += inner_gaps as i32;
                geo.y += inner_gaps as i32;
                geo.width -= inner_gaps * 2;
                geo.height -= inner_gaps * 2;
            }
        }

        geos
    }
}

/// A [`LayoutGenerator`] that lays out windows in a spiral.
///
/// This is similar to the [`DwindleLayout`] but in a spiral instead of
/// towards the bottom right corner.
#[derive(Clone, Debug, PartialEq)]
pub struct SpiralLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The ratio for each dwindle split.
    ///
    /// The first split will use the factor at key `1`,
    /// the second at key `2`, and so on.
    ///
    /// Splits without a factor will default to 0.5.
    pub split_factors: HashMap<usize, f32>,
}

impl Default for SpiralLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            split_factors: Default::default(),
        }
    }
}

impl LayoutGenerator for SpiralLayout {
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
        let win_count = args.windows.len() as u32;

        if win_count == 0 {
            return Vec::new();
        }

        let width = args.output_width;
        let height = args.output_height;

        let mut geos = Vec::<Geometry>::new();

        let (outer_gaps, inner_gaps) = match self.gaps {
            Gaps::Absolute(gaps) => (gaps, None),
            Gaps::Split { inner, outer } => (outer, Some(inner)),
        };

        let gaps = match inner_gaps {
            Some(_) => 0,
            None => outer_gaps,
        };

        let mut rect = Geometry {
            x: 0,
            y: 0,
            width,
            height,
        }
        .split_at(Axis::Horizontal, 0, outer_gaps)
        .0
        .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
        .0
        .split_at(Axis::Vertical, 0, outer_gaps)
        .0
        .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
        .0;

        if win_count == 1 {
            geos.push(rect)
        } else {
            for i in 1..win_count {
                let factor = self
                    .split_factors
                    .get(&(i as usize))
                    .copied()
                    .unwrap_or(0.5)
                    .clamp(0.1, 0.9);

                let (axis, mut split_coord) = if i % 2 == 1 {
                    (Axis::Vertical, rect.x + (rect.width as f32 * factor) as i32)
                } else {
                    (
                        Axis::Horizontal,
                        rect.y + (rect.height as f32 * factor) as i32,
                    )
                };
                split_coord -= gaps as i32 / 2;

                let (to_push, rest) = if let 1 | 2 = i % 4 {
                    let (to_push, rest) = rect.split_at(axis, split_coord, gaps);
                    (Some(to_push), rest)
                } else {
                    let (rest, to_push) = rect.split_at(axis, split_coord, gaps);
                    (to_push, Some(rest))
                };

                if let Some(to_push) = to_push {
                    geos.push(to_push);
                }

                if let Some(rest) = rest {
                    rect = rest;
                } else {
                    break;
                }
            }

            geos.push(rect)
        }

        if let Some(inner_gaps) = inner_gaps {
            for geo in geos.iter_mut() {
                geo.x += inner_gaps as i32;
                geo.y += inner_gaps as i32;
                geo.width -= inner_gaps * 2;
                geo.height -= inner_gaps * 2;
            }
        }

        geos
    }
}

/// Which corner the corner window will in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerLocation {
    /// The corner window will be in the top left.
    TopLeft,
    /// The corner window will be in the top right.
    TopRight,
    /// The corner window will be in the bottom left.
    BottomLeft,
    /// The corner window will be in the bottom right.
    BottomRight,
}

/// A [`LayoutGenerator`] that has one main corner window and a
/// horizontal and vertical stack flanking it on the other two sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerLayout {
    /// Gaps between windows.
    ///
    /// Defaults to `Gaps::Absolute(8)`.
    pub gaps: Gaps,
    /// The proportion of the output that the width of the window takes up.
    ///
    /// Defaults to 0.5.
    pub corner_width_factor: f32,
    /// The proportion of the output that the height of the window takes up.
    ///
    /// Defaults to 0.5.
    pub corner_height_factor: f32,
    /// The location of the corner window.
    pub corner_loc: CornerLocation,
}

impl Default for CornerLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            corner_width_factor: 0.5,
            corner_height_factor: 0.5,
            corner_loc: CornerLocation::TopLeft,
        }
    }
}

impl LayoutGenerator for CornerLayout {
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
        let win_count = args.windows.len() as u32;

        if win_count == 0 {
            return Vec::new();
        }

        let width = args.output_width;
        let height = args.output_height;

        let mut geos = Vec::<Geometry>::new();

        let (outer_gaps, inner_gaps) = match self.gaps {
            Gaps::Absolute(gaps) => (gaps, None),
            Gaps::Split { inner, outer } => (outer, Some(inner)),
        };

        let gaps = match inner_gaps {
            Some(_) => 0,
            None => outer_gaps,
        };

        let rect = Geometry {
            x: 0,
            y: 0,
            width,
            height,
        }
        .split_at(Axis::Horizontal, 0, outer_gaps)
        .0
        .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
        .0
        .split_at(Axis::Vertical, 0, outer_gaps)
        .0
        .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
        .0;

        if win_count == 1 {
            geos.push(rect)
        } else {
            let (mut corner_rect, vert_stack_rect) = match self.corner_loc {
                CornerLocation::TopLeft | CornerLocation::BottomLeft => {
                    let x_slice_point = rect.x
                        + (rect.width as f32 * self.corner_width_factor).round() as i32
                        - gaps as i32 / 2;
                    let (corner_rect, vert_stack_rect) =
                        rect.split_at(Axis::Vertical, x_slice_point, gaps);
                    (Some(corner_rect), vert_stack_rect)
                }
                CornerLocation::TopRight | CornerLocation::BottomRight => {
                    let x_slice_point = rect.x
                        + (rect.width as f32 * (1.0 - self.corner_width_factor)).round() as i32
                        - gaps as i32 / 2;
                    let (vert_stack_rect, corner_rect) =
                        rect.split_at(Axis::Vertical, x_slice_point, gaps);
                    (corner_rect, Some(vert_stack_rect))
                }
            };

            if win_count == 2 {
                geos.extend([corner_rect, vert_stack_rect].into_iter().flatten());
            } else {
                let horiz_stack_rect = match self.corner_loc {
                    CornerLocation::TopLeft | CornerLocation::TopRight => {
                        let y_slice_point = rect.y
                            + (rect.height as f32 * self.corner_height_factor).round() as i32
                            - gaps as i32 / 2;

                        corner_rect.and_then(|corner| {
                            let (corner, horiz) =
                                corner.split_at(Axis::Horizontal, y_slice_point, gaps);
                            corner_rect = Some(corner);
                            horiz
                        })
                    }
                    CornerLocation::BottomLeft | CornerLocation::BottomRight => {
                        let y_slice_point = rect.y
                            + (rect.height as f32 * (1.0 - self.corner_height_factor)).round()
                                as i32
                            - gaps as i32 / 2;

                        corner_rect.map(|corner| {
                            let (horiz, corner) =
                                corner.split_at(Axis::Horizontal, y_slice_point, gaps);
                            corner_rect = corner;
                            horiz
                        })
                    }
                };

                if let (Some(mut horiz_stack_rect), Some(mut vert_stack_rect), Some(corner_rect)) =
                    (horiz_stack_rect, vert_stack_rect, corner_rect)
                {
                    geos.push(corner_rect);

                    let mut vert_geos = Vec::new();
                    let mut horiz_geos = Vec::new();

                    let vert_stack_count = ((win_count - 1) as f32 / 2.0).ceil() as i32;
                    let horiz_stack_count = ((win_count - 1) as f32 / 2.0).floor() as i32;

                    let vert_stack_y = vert_stack_rect.y;
                    let vert_win_height = vert_stack_rect.height as f32 / vert_stack_count as f32;

                    for i in 1..vert_stack_count {
                        let slice_point = vert_stack_y
                            + (vert_win_height * i as f32).round() as i32
                            - gaps as i32 / 2;

                        let (to_push, rest) =
                            vert_stack_rect.split_at(Axis::Horizontal, slice_point, gaps);

                        vert_geos.push(to_push);

                        if let Some(rest) = rest {
                            vert_stack_rect = rest;
                        } else {
                            break;
                        }
                    }

                    vert_geos.push(vert_stack_rect);

                    let horiz_stack_x = horiz_stack_rect.x;
                    let horiz_win_width = horiz_stack_rect.width as f32 / horiz_stack_count as f32;

                    for i in 1..horiz_stack_count {
                        let slice_point = horiz_stack_x
                            + (horiz_win_width * i as f32).round() as i32
                            - gaps as i32 / 2;

                        let (to_push, rest) =
                            horiz_stack_rect.split_at(Axis::Vertical, slice_point, gaps);

                        horiz_geos.push(to_push);

                        if let Some(rest) = rest {
                            horiz_stack_rect = rest;
                        } else {
                            break;
                        }
                    }

                    horiz_geos.push(horiz_stack_rect);

                    for i in 0..(vert_geos.len() + horiz_geos.len()) {
                        if i % 2 == 0 {
                            geos.push(vert_geos[i / 2]);
                        } else {
                            geos.push(horiz_geos[i / 2]);
                        }
                    }
                }
            }
        }

        if let Some(inner_gaps) = inner_gaps {
            for geo in geos.iter_mut() {
                geo.x += inner_gaps as i32;
                geo.y += inner_gaps as i32;
                geo.width -= inner_gaps * 2;
                geo.height -= inner_gaps * 2;
            }
        }

        geos
    }
}

/// A [`LayoutGenerator`] that attempts to layout windows such that
/// they are the same size.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FairLayout {
    /// The proportion of the output that the width of the window takes up.
    ///
    /// Defaults to 0.5.
    pub gaps: Gaps,
    /// Which axis the lines of windows will run.
    ///
    /// Defaults to [`Axis::Vertical`].
    pub axis: Axis,
}

impl Default for FairLayout {
    fn default() -> Self {
        Self {
            gaps: Gaps::Absolute(8),
            axis: Axis::Vertical,
        }
    }
}

impl LayoutGenerator for FairLayout {
    fn layout(&self, args: &LayoutArgs) -> Vec<Geometry> {
        let win_count = args.windows.len() as u32;

        if win_count == 0 {
            return Vec::new();
        }

        let width = args.output_width;
        let height = args.output_height;

        let mut geos = Vec::<Geometry>::new();

        let (outer_gaps, inner_gaps) = match self.gaps {
            Gaps::Absolute(gaps) => (gaps, None),
            Gaps::Split { inner, outer } => (outer, Some(inner)),
        };

        let gaps = match inner_gaps {
            Some(_) => 0,
            None => outer_gaps,
        };

        let mut rect = Geometry {
            x: 0,
            y: 0,
            width,
            height,
        }
        .split_at(Axis::Horizontal, 0, outer_gaps)
        .0
        .split_at(Axis::Horizontal, (height - outer_gaps) as i32, outer_gaps)
        .0
        .split_at(Axis::Vertical, 0, outer_gaps)
        .0
        .split_at(Axis::Vertical, (width - outer_gaps) as i32, outer_gaps)
        .0;

        if win_count == 1 {
            geos.push(rect);
        } else if win_count == 2 {
            let len = match self.axis {
                Axis::Vertical => rect.width,
                Axis::Horizontal => rect.height,
            };

            let coord = match self.axis {
                Axis::Vertical => rect.x,
                Axis::Horizontal => rect.y,
            };

            let (rect1, rect2) =
                rect.split_at(self.axis, coord + len as i32 / 2 - gaps as i32 / 2, gaps);

            geos.push(rect1);
            if let Some(rect2) = rect2 {
                geos.push(rect2);
            }
        } else {
            let line_count = (win_count as f32).sqrt().round() as u32;

            let mut wins_per_line = Vec::new();

            let max_per_line = if win_count > line_count * line_count {
                line_count + 1
            } else {
                line_count
            };

            for i in 1..=win_count {
                let index = (i as f32 / max_per_line as f32).ceil() as usize - 1;
                if wins_per_line.get(index).is_none() {
                    wins_per_line.push(0);
                }
                wins_per_line[index] += 1;
            }

            assert_eq!(wins_per_line.len(), line_count as usize);

            let mut line_rects = Vec::new();

            let (coord, len, axis) = match self.axis {
                Axis::Horizontal => (
                    rect.y,
                    rect.height as f32 / line_count as f32,
                    Axis::Horizontal,
                ),
                Axis::Vertical => (
                    rect.x,
                    rect.width as f32 / line_count as f32,
                    Axis::Vertical,
                ),
            };

            for i in 1..line_count {
                let slice_point = coord + (len * i as f32) as i32 - gaps as i32 / 2;
                let (to_push, rest) = rect.split_at(axis, slice_point, gaps);
                line_rects.push(to_push);
                if let Some(rest) = rest {
                    rect = rest;
                } else {
                    break;
                }
            }

            line_rects.push(rect);

            for (i, mut line_rect) in line_rects.into_iter().enumerate() {
                let (coord, len, axis) = match self.axis {
                    Axis::Vertical => (
                        line_rect.y,
                        line_rect.height as f32 / wins_per_line[i] as f32,
                        Axis::Horizontal,
                    ),
                    Axis::Horizontal => (
                        line_rect.x,
                        line_rect.width as f32 / wins_per_line[i] as f32,
                        Axis::Vertical,
                    ),
                };

                for j in 1..wins_per_line[i] {
                    let slice_point = coord + (len * j as f32) as i32 - gaps as i32 / 2;
                    let (to_push, rest) = line_rect.split_at(axis, slice_point, gaps);
                    geos.push(to_push);
                    if let Some(rest) = rest {
                        line_rect = rest;
                    } else {
                        break;
                    }
                }

                geos.push(line_rect);
            }
        }

        if let Some(inner_gaps) = inner_gaps {
            for geo in geos.iter_mut() {
                geo.x += inner_gaps as i32;
                geo.y += inner_gaps as i32;
                geo.width -= inner_gaps * 2;
                geo.height -= inner_gaps * 2;
            }
        }

        geos
    }
}
