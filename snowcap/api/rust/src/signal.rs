//! Widget signals.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

/// Retention policy for signal handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerPolicy {
    /// Keep the handler.
    Keep,
    /// Discard the handler.
    Discard,
}

/// A handle to disconnect a signal handler.
pub struct Handle<S>(Weak<dyn Fn(S) -> HandlerPolicy + Sync + Send>);

/// Internal type to hold a collection of handlers.
#[derive(Default, Clone)]
struct SignalEntry<S> {
    callbacks: Vec<Arc<dyn Fn(S) -> HandlerPolicy + Sync + Send>>,
}

/// A typed signal manager.
///
/// [`Signaler`]s holds handlers for signals in a type-erased way. Other types can
/// [connect] and [disconnect] handlers, or [emit] signals.
///
/// [connect]: Signaler::connect
/// [disconnect]: Signaler::disconnect
/// [emit]: Signaler::emit
#[derive(Default, Clone, Debug)]
pub struct Signaler {
    entries: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Sync + Send>>>>,
}

impl Signaler {
    /// Creates a new default [`Signaler`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Connects a signal handler.
    ///
    /// This handler will be called when the given type `S` is emitted.
    ///
    /// All handlers return a [`HandlerPolicy`] to determine whether to remove the handler
    /// afterward.
    pub fn connect<S, F>(&mut self, callback: F) -> Handle<S>
    where
        S: Clone + 'static,
        F: Fn(S) -> HandlerPolicy + Sync + Send + 'static,
    {
        let mut entries = self.entries.lock().unwrap();

        let key = TypeId::of::<S>();

        let entry = entries
            .entry(key)
            .or_insert_with(|| Box::new(SignalEntry::<S>::new()))
            .downcast_mut::<SignalEntry<S>>()
            .expect("Could not retrieve entry");

        entry.add_callback(callback)
    }

    /// Disconnects the signal handler referred to by `handle`.
    pub fn disconnect<S>(&mut self, handle: Handle<S>)
    where
        S: Clone + 'static,
    {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = Self::get_entry(&mut entries) {
            entry.remove_callback(handle);
        }
    }

    /// Disconnects all handlers for a specific signal type.
    pub fn disconnect_all_for<S>(&mut self)
    where
        S: Clone + 'static,
    {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = Self::get_entry::<S>(&mut entries) {
            entry.clear()
        }
    }

    /// Disconnects all handlers.
    pub fn disconnect_all(&mut self) {
        self.entries.lock().unwrap().clear()
    }

    /// Emits a signal.
    ///
    /// This will call all handlers connected for type `S`.
    pub fn emit<S>(&self, signal: S)
    where
        S: Clone + 'static,
    {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = Self::get_entry(&mut entries) {
            entry.emit(signal)
        }
    }

    /// Returns the [`SignalEntry`] for a given type.
    fn get_entry<S>(
        entries: &mut HashMap<TypeId, Box<dyn Any + Sync + Send>>,
    ) -> Option<&mut SignalEntry<S>>
    where
        S: Clone + 'static,
    {
        entries
            .get_mut(&TypeId::of::<S>())
            .and_then(|entry| entry.downcast_mut::<SignalEntry<S>>())
    }
}

impl<S> SignalEntry<S>
where
    S: Clone,
{
    /// Creates a new [`SignalEntry`].
    fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Adds a callback to this [`SignalEntry`] and returns a [`Handle`] to that callback.
    fn add_callback<F>(&mut self, callback: F) -> Handle<S>
    where
        F: Fn(S) -> HandlerPolicy + Sync + Send + 'static,
    {
        let callback: Arc<dyn Fn(S) -> HandlerPolicy + Sync + Send> = Arc::new(callback);

        let ret = Handle(Arc::downgrade(&callback));

        self.callbacks.push(callback);

        ret
    }

    /// Removes the callback the `handle` refers to.
    fn remove_callback(&mut self, handle: Handle<S>) {
        let Some(ptr) = handle.0.upgrade() else {
            return;
        };

        self.callbacks.retain(|cb| !Arc::ptr_eq(&ptr, cb));
    }

    /// Removes all handlers.
    fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// Emits the `signal` by calling every handler, and removes the ones that need to be discarded.
    fn emit(&mut self, signal: S) {
        self.callbacks
            .retain_mut(|cb| cb(signal.clone()) == HandlerPolicy::Keep);
    }
}
