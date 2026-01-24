use std::any::Any;

/// A universal message type.
///
/// This is a suitable catch-all message for all built-in widget programs.
/// Types must implement [`Universal`] in order to be converted to and from this message.
#[derive(Debug, Clone)]
pub struct UniversalMsg(Box<dyn Value>);

trait Value: dyn_clone::DynClone + Any + Send {}
dyn_clone::clone_trait_object!(Value);

impl<T: Clone + Send + Any> Value for T {}

impl std::fmt::Debug for dyn Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value").finish_non_exhaustive()
    }
}

/// A trait that allows types to be converted to and from the [`UniversalMsg`].
pub trait Universal: Clone + Send + 'static {
    fn to_universal(self) -> UniversalMsg {
        UniversalMsg(Box::new(self) as _)
    }
    fn from_universal(msg: UniversalMsg) -> Option<Self> {
        let msg = msg.0 as Box<dyn Any>;
        msg.downcast::<Self>().ok().map(|inner| *inner)
    }
}

impl<T: Universal> From<T> for UniversalMsg {
    fn from(value: T) -> Self {
        value.to_universal()
    }
}

impl<T: Universal> From<UniversalMsg> for Option<T> {
    fn from(value: UniversalMsg) -> Self {
        T::from_universal(value)
    }
}
