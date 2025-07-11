//! Utilities for converting to and from API types

pub trait FromApi<T> {
    fn from_api(api_type: T) -> Self;
}
