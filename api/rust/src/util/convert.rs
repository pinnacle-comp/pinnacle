// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Utilities for conversion (primarily from and to API types).

// CONVERSIONS: Native -> Api

/// An API type that is constructible from a native type.
pub trait FromNative<NativeType> {
    /// Create the API type from the native type.
    fn from_native(native_value: NativeType) -> Self;
}

/// An API type that is possibly constructible from a native type.
pub trait TryFromNative<NativeType>: Sized {
    /// Error type for the conversion.
    type Error;

    /// Attempt to convert the native type into this API type.
    fn try_from_native(native_value: NativeType) -> Result<Self, Self::Error>;
}

/// Native type convertible into some API type
///
/// Prefer implementing [`FromNative`] on the API type.
pub trait IntoApi<ApiType> {
    /// Convert this type into an api type.
    fn into_api(self) -> ApiType;
}

impl<Native, Api: FromNative<Native>> IntoApi<Api> for Native {
    fn into_api(self) -> Api {
        Api::from_native(self)
    }
}

/// Native type possibly convertible into some API type
///
/// Prefer implementing [`TryFromNative`]
pub trait TryIntoApi<ApiType> {
    /// Error type for the conversion.
    type Error;
    /// Try to convert this type into an API type
    fn try_into_api(self) -> Result<ApiType, Self::Error>;
}

impl<Native, Api: TryFromNative<Native>> TryIntoApi<Api> for Native {
    type Error = Api::Error;

    fn try_into_api(self) -> Result<Api, Self::Error> {
        Api::try_from_native(self)
    }
}

// CONVERSIONS: Api -> Native

/// A native type that is constructible from an API type.
pub trait FromApi<ApiType> {
    /// Create the native type from the api type.
    fn from_api(api_value: ApiType) -> Self;
}

/// An native type that is possibly constructible from an API type.
pub trait TryFromApi<ApiType>: Sized {
    /// Error type for the conversion.
    type Error;

    /// Attempt to convert the API type into this native type.
    fn try_from_api(api_value: ApiType) -> Result<Self, Self::Error>;
}

/// API type convertible into some native type
///
/// Prefer implementing [`FromApi`] on the native type.
pub trait IntoNative<NativeType> {
    /// Convert this API type into a native type.
    fn into_native(self) -> NativeType;
}

impl<Api, Native: FromApi<Api>> IntoNative<Native> for Api {
    fn into_native(self) -> Native {
        Native::from_api(self)
    }
}

/// API type possibly convertible into some native type
///
/// Prefer implementing [`TryFromApi`]
pub trait TryIntoNative<NativeType> {
    /// Error type for the conversion.
    type Error;
    /// Try to convert this type into a native type
    fn try_into_native(self) -> Result<NativeType, Self::Error>;
}

impl<Api, Native: TryFromApi<Api>> TryIntoNative<Native> for Api {
    type Error = Native::Error;

    fn try_into_native(self) -> Result<Native, Self::Error> {
        Native::try_from_api(self)
    }
}
