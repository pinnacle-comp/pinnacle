// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Utility types.

use std::pin::Pin;

use futures::{Future, StreamExt, stream::FuturesOrdered};

use crate::BlockOnTokio;
pub use crate::batch_boxed;
pub use crate::batch_boxed_async;

/// A horizontal or vertical axis.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum Axis {
    /// A horizontal axis.
    Horizontal,
    /// A vertical axis.
    Vertical,
}

/// A cardinal direction.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum Direction {
    /// The left/west direction.
    Left,
    /// The right/east direction.
    Right,
    /// The up/north direction.
    Up,
    /// The down/south direction.
    Down,
}

/// Batches a set of requests that will be sent to the compositor all at once.
///
/// # Rationale
///
/// Normally, all API calls are blocking. For example, calling [`window::get_all`][crate::window::get_all]
/// then calling [`WindowHandle::app_id`][crate::window::WindowHandle::app_id]
/// on each returned window handle will block after each `app_id` call waiting for the compositor to respond.
///
/// In order to mitigate this issue, you can batch up a set of API calls using this function.
/// This will send all requests to the compositor at once without blocking, then wait for the compositor
/// to respond.
///
/// You'll see that this function takes in an `IntoIterator` of `Future`s. As such,
/// most API calls that return something have an async variant named `*_async` that returns a future.
/// You must pass these futures into the batch function instead of their non-async counterparts.
///
/// # The `batch_boxed` macro
/// The [`util`][crate::util] module also provides the [`batch_boxed`] macro.
///
/// The [`batch`] function only accepts one concrete type of future, meaning that you
/// can only batch a collection of futures from one specific function or method.
///
/// As a convenience, `batch_boxed` accepts one or more different futures that return the same type.
/// It will place provided futures in a `Pin<Box<_>>` to erase the types and pass them along to `batch`.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::util::batch;
/// # use pinnacle_api::window;
/// let windows = window::get_all().collect::<Vec<_>>();
/// let props: Vec<String> = batch(windows.iter().map(|window| window.app_id_async()));
/// ```
///
pub fn batch<T>(requests: impl IntoIterator<Item = impl Future<Output = T>>) -> Vec<T> {
    batch_async(requests).block_on_tokio()
}

/// The async version of [`batch`].
///
/// See [`batch`] for more information.
pub async fn batch_async<T>(requests: impl IntoIterator<Item = impl Future<Output = T>>) -> Vec<T> {
    let results = FuturesOrdered::from_iter(requests).collect::<Vec<_>>();
    results.await
}

/// Batches API calls in different concrete futures.
///
/// The [`batch`] function only accepts a collection of the same concrete future e.g.
/// from a single async function or method.
///
/// To support different futures (that still return the same type), this macro will place provided
/// futures in a `Pin<Box<_>>` to erase their type and pass them along to `batch`.
///
/// # Examples
///
/// ```no_run
/// # use pinnacle_api::util::batch_boxed;
/// # use pinnacle_api::window;
/// # || {
/// let mut windows = window::get_all();
/// let first = windows.next()?;
/// let last = windows.last()?;
///
/// let classes: Vec<String> = batch_boxed![
///     async {
///         let class = first.app_id_async().await;
///         class
///     },
///     async {
///         let mut class = last.app_id_async().await;
///         class += "hello";
///         class
///     },
/// ];
/// # Some(())
/// # };
/// ```
#[macro_export]
macro_rules! batch_boxed {
    [ $($request:expr),* $(,)? ] => {
        $crate::util::batch([
            $(
                ::std::boxed::Box::pin($request) as ::std::pin::Pin<::std::boxed::Box<dyn std::future::Future<Output = _>>>,
            )*
        ])
    };
}

/// The async version of [`batch_boxed`].
///
/// See [`batch_boxed`] for more information.
#[macro_export]
macro_rules! batch_boxed_async {
    [ $first:expr, $($request:expr),* ] => {
        $crate::util::batch_async([
            ::std::boxed::Box::pin($first) as ::std::pin::Pin<::std::boxed::Box<dyn std::future::Future<Output = _>>>,
            $(
                ::std::boxed::Box::pin($request),
            )*
        ])
    };
}

/// Methods for batch sending API requests to the compositor.
pub trait Batch<I> {
    /// [`batch_map`][Batch::batch_map]s then finds the object for which `find` with the results
    /// of awaiting `map_to_future(item)` returns `true`.
    fn batch_find<M, F, FutOp>(self, map_to_future: M, find: F) -> Option<I>
    where
        Self: Sized,
        M: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>,
        F: FnMut(&FutOp) -> bool;

    /// Maps the collection to compositor requests, batching all calls.
    fn batch_map<F, FutOp>(self, map: F) -> impl Iterator<Item = FutOp>
    where
        Self: Sized,
        F: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>;

    /// [`batch_map`][Batch::batch_map]s then filters for objects for which `predicate` with the
    /// results of awaiting `map_to_future(item)` returns `true`.
    fn batch_filter<M, F, FutOp>(self, map_to_future: M, predicate: F) -> impl Iterator<Item = I>
    where
        Self: Sized,
        M: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>,
        F: FnMut(FutOp) -> bool;
}

impl<T: IntoIterator<Item = I>, I> Batch<I> for T {
    fn batch_find<M, F, FutOp>(self, map_to_future: M, mut find: F) -> Option<I>
    where
        Self: Sized,
        M: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>,
        F: FnMut(&FutOp) -> bool,
    {
        let items = self.into_iter().collect::<Vec<_>>();
        let futures = items.iter().map(map_to_future);
        let results = crate::util::batch(futures);

        assert_eq!(items.len(), results.len());

        items
            .into_iter()
            .zip(results)
            .find(|(_, fut_op)| find(fut_op))
            .map(|(item, _)| item)
    }

    fn batch_map<F, FutOp>(self, map: F) -> impl Iterator<Item = FutOp>
    where
        Self: Sized,
        F: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>,
    {
        let items = self.into_iter().collect::<Vec<_>>();
        let futures = items.iter().map(map);
        crate::util::batch(futures).into_iter()
    }

    fn batch_filter<M, F, FutOp>(
        self,
        map_to_future: M,
        mut predicate: F,
    ) -> impl Iterator<Item = I>
    where
        Self: Sized,
        M: for<'a> FnMut(&'a I) -> Pin<Box<dyn Future<Output = FutOp> + 'a>>,
        F: FnMut(FutOp) -> bool,
    {
        let items = self.into_iter().collect::<Vec<_>>();
        let futures = items.iter().map(map_to_future);
        let results = crate::util::batch(futures);

        assert_eq!(items.len(), results.len());

        items
            .into_iter()
            .zip(results)
            .filter_map(move |(item, fut_op)| predicate(fut_op).then_some(item))
    }
}

/// A point in space.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct Point {
    /// The x-coordinate.
    pub x: i32,
    /// The y-coordinate.
    pub y: i32,
}

impl From<Point> for pinnacle_api_defs::pinnacle::util::v1::Point {
    fn from(value: Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

/// A size with a width and height.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct Size {
    /// The width.
    pub w: u32,
    /// The height.
    pub h: u32,
}

impl From<Size> for pinnacle_api_defs::pinnacle::util::v1::Size {
    fn from(value: Size) -> Self {
        Self {
            width: value.w,
            height: value.h,
        }
    }
}

/// A rectangle with a location and size.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct Rect {
    /// The location.
    pub loc: Point,
    /// The size.
    pub size: Size,
}

impl From<Rect> for pinnacle_api_defs::pinnacle::util::v1::Rect {
    fn from(value: Rect) -> Self {
        Self {
            loc: Some(value.loc.into()),
            size: Some(value.size.into()),
        }
    }
}
