// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Utility types.

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
