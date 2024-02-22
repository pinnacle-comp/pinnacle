// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Utility types.

use std::pin::Pin;

use futures::{stream::FuturesOrdered, Future, StreamExt};

use crate::block_on_tokio;

pub use crate::batch_boxed;
pub use crate::batch_boxed_async;

/// The size and location of something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Geometry {
    /// The x position
    pub x: i32,
    /// The y position
    pub y: i32,
    /// The width
    pub width: u32,
    /// The height
    pub height: u32,
}

/// Batch a set of requests that will be sent ot the compositor all at once.
///
/// # Rationale
///
/// Normally, all API calls are blocking. For example, calling [`Window::get_all`][crate::window::Window::get_all]
/// then calling [`WindowHandle.props`][crate::window::WindowHandle::props]
/// on each returned window handle will block after each `props` call waiting for the compositor to respond:
///
/// ```
/// // This will block after each call to `window.props()`, meaning this all happens synchronously.
/// // If the compositor is running slowly for whatever reason, this will take a long time to complete.
/// let props = window.get_all()
///     .map(|window| window.props())
///     .collect::<Vec<_>>();
/// ```
///
/// In order to mitigate this issue, you can batch up a set of API calls using this function.
/// This will send all requests to the compositor at once without blocking, then wait for the compositor
/// to respond.
///
/// You'll see that this function takes in an `IntoIterator` of `Future`s. As such,
/// all API calls that return something have an async variant named `*_async` that returns a future.
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
/// ```
/// use pinnacle_api::util::batch;
/// use pinnacle_api::window::WindowProperties;
///
/// let props: Vec<WindowProperties> = batch(window.get_all().map(|window| window.props_async()));
///                                                         // Don't forget the `async` ^^^^^
/// ```
///
pub fn batch<T>(requests: impl IntoIterator<Item = impl Future<Output = T>>) -> Vec<T> {
    block_on_tokio(batch_async(requests))
}

/// The async version of [`batch`].
///
/// See [`batch`] for more information.
pub async fn batch_async<T>(requests: impl IntoIterator<Item = impl Future<Output = T>>) -> Vec<T> {
    let results = FuturesOrdered::from_iter(requests).collect::<Vec<_>>();
    results.await
}

/// A convenience macro to batch API calls in different concrete futures.
///
/// The [`batch`] function only accepts a collection of the same concrete future e.g.
/// from a single async function or method.
///
/// To support different futures (that still return the same value), this macro will place provided
/// futures in a `Pin<Box<_>>` to erase their type and pass them along to `batch`.
///
/// # Examples
/// ```
/// use pinnacle_api::util::batch_boxed;
///
/// let mut windows = window.get_all();
/// let first = windows.next()?;
/// let last = windows.last()?;
///
/// let classes: Vec<String> = batch_boxed![
///     async {
///         let class = first.class_async().await;
///         class.unwrap_or("no class".to_string())
///     },
///     async {
///         let mut class = last.class_async().await.unwrap_or("alalala");
///         class += "hello";
///         class
///     },
/// ];
/// ```
#[macro_export]
macro_rules! batch_boxed {
    [ $first:expr, $($request:expr),* ] => {
        $crate::util::batch([
            ::std::boxed::Box::pin($first) as ::std::pin::Pin<::std::boxed::Box<dyn std::future::Future<Output = _>>>,
            $(
                ::std::boxed::Box::pin($request),
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
    /// [`batch_map`][Batch::batch_map]s then finds the object for which `f` with the results
    /// returns `true`.
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
}
