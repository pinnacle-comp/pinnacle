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
