//! Utility types & function for widgets.

/// Represents an angle in degrees.
#[derive(Default, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl From<f32> for Degrees {
    fn from(degree: f32) -> Self {
        Self(degree)
    }
}

impl From<Radians> for Degrees {
    fn from(radians: Radians) -> Self {
        Self(radians.0.to_degrees())
    }
}

/// Represents an angle expressed in radians.
#[derive(Default, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl From<f32> for Radians {
    fn from(radians: f32) -> Self {
        Self(radians)
    }
}

impl From<Degrees> for Radians {
    fn from(degrees: Degrees) -> Self {
        Self(degrees.0.to_radians())
    }
}
