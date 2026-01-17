//! Utilities for converting to and from API types

pub trait FromApi<T> {
    fn from_api(api_type: T) -> Self;
}

pub trait TryFromApi<T>: Sized {
    type Error;

    fn try_from_api(api_type: T) -> Result<Self, Self::Error>;
}
