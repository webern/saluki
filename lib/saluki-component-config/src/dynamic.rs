//! Typed, scoped runtime configuration handles.
//!
//! A [`ScopedConfigHandle`] gives a component a read-only, typed view of *one* configuration slice
//! that the configuration system refreshes at runtime. The handle never exposes a configuration map
//! and never names a Datadog Agent key: the component observes a native slice (for example a
//! forwarder configuration) and nothing else, so a change to another component's configuration
//! physically cannot reach it.
//!
//! The type lives in this leaf crate, rather than in the configuration system, so that *library*
//! components in `saluki-components` can consume it directly. The dependency arrow points down toward
//! this leaf; a handle defined in the configuration system could only be consumed by the binary,
//! which is the limitation this placement removes. The configuration system constructs handles with
//! [`ScopedConfigHandle::new`] and owns the sending half.
//!
//! Two consumption idioms are supported on the same handle:
//!
//! - *Reactive*: await [`ScopedConfigHandle::changed`] in a `select!` arm and apply the new value
//!   when a change arrives (used by components that must rebuild state on change).
//! - *Latest-read*: call [`ScopedConfigHandle::borrow`] at the point of use to read the freshest
//!   value with no clone (used by components that consult the value when they need it, such as the
//!   forwarder reading the current API key when building a request).

use tokio::sync::watch;

/// A scoped, typed handle a single component holds to observe updates to *its* config slice.
///
/// Because the underlying channel only ever carries this slice's type, the component cannot observe
/// another component's configuration.
#[derive(Clone, Debug)]
pub struct ScopedConfigHandle<T> {
    rx: watch::Receiver<T>,
}

impl<T> ScopedConfigHandle<T> {
    /// Creates a handle from the receiving half of a watch channel.
    ///
    /// The configuration system owns the sending half and pushes refreshed slices onto it.
    pub fn new(rx: watch::Receiver<T>) -> Self {
        Self { rx }
    }

    /// Borrows the latest value for this slice without cloning.
    ///
    /// This is the latest-read idiom: a component reads the freshest value at its point of use. The
    /// returned guard holds a read lock on the channel, so it should be dropped promptly.
    pub fn borrow(&self) -> watch::Ref<'_, T> {
        self.rx.borrow()
    }

    /// Returns whether this slice has an unobserved change pending.
    pub fn has_changed(&self) -> bool {
        self.rx.has_changed().unwrap_or(false)
    }
}

impl<T: Clone> ScopedConfigHandle<T> {
    /// Returns a clone of the current value for this slice.
    pub fn current(&self) -> T {
        self.rx.borrow().clone()
    }

    /// Waits for the next change to this slice and returns the new value.
    ///
    /// This is the reactive idiom. Returns `None` if the configuration system has shut down.
    pub async fn changed(&mut self) -> Option<T> {
        match self.rx.changed().await {
            Ok(()) => Some(self.rx.borrow().clone()),
            Err(_) => None,
        }
    }
}
