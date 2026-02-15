use std::any::Any;

pub use snowcap_derive::Universal;

/// A universal message type.
///
/// This is a suitable catch-all message for all built-in widget programs.
/// Types must implement [`Universal`] in order to be converted into and from this message.
#[derive(Debug, Clone)]
pub struct UniversalMsg(Box<dyn Value>);

impl UniversalMsg {
    /// Returns `true` if this `UniversalMsg` is of type `T`.
    pub fn is<T: 'static>(&self) -> bool {
        (&*self.0 as &dyn Any).is::<T>()
    }

    /// Attempts to downcast this `UniversalMsg` to a concrete type.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        let value = self.0;
        if (&*value as &dyn Any).is::<T>() {
            let ret = (value as Box<dyn Any>)
                .downcast::<T>()
                .expect("checked above");
            Ok(*ret)
        } else {
            Err(Self(value))
        }
    }

    /// Returns a reference to the inner value of it is of type `T`,
    /// or `None` if it isn't.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (&*self.0 as &dyn Any).downcast_ref()
    }

    /// Returns a mutable reference to the inner value of it is of type `T`,
    /// or `None` if it isn't.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (&mut *self.0 as &mut dyn Any).downcast_mut()
    }
}

trait Value: dyn_clone::DynClone + Any + Send {}
dyn_clone::clone_trait_object!(Value);

impl<T: Clone + Send + Any> Value for T {}

impl std::fmt::Debug for dyn Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value").finish_non_exhaustive()
    }
}

/// A trait that allows types to be converted into and from the [`UniversalMsg`].
pub trait Universal: Clone + Send + 'static {
    // Converts this type into a `UniversalMsg`.
    fn into_universal(self) -> UniversalMsg {
        UniversalMsg(Box::new(self) as _)
    }
    // Attempts to convert a `UniversalMsg` into this type.
    fn from_universal(msg: UniversalMsg) -> Result<Self, UniversalMsg> {
        msg.downcast()
    }
}

impl<T: Universal> From<T> for UniversalMsg {
    fn from(value: T) -> Self {
        value.into_universal()
    }
}

impl<T: Universal> From<UniversalMsg> for Option<T> {
    fn from(value: UniversalMsg) -> Self {
        T::from_universal(value).ok()
    }
}
