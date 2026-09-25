//! State the client shares with its worker, which outlives any one run of the worker.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Mutex;

use tokio::sync::Notify;
use uuid::Uuid;

use crate::subscription::Publisher;
use crate::{ConfigId, Error, ProductDecoder, Result, Subscription};

/// The client's identity and subscriptions, shared by every client handle and the worker.
///
/// A worker restart discards protocol state but not this, so a restart never strands a subscriber or changes the ID the
/// Agent tracks the client by.
pub(crate) struct Shared {
    /// The ID the client reports in every poll, generated once per client.
    // TODO: remove dead_code guard when the worker builds requests.
    #[allow(dead_code)]
    pub(crate) client_id: String,

    /// Each subscribed product's publisher, keyed by its protocol string so that a [`ProductId`] and its string are one
    /// product.
    ///
    /// An entry whose subscriptions have all been dropped stays until it is replaced by a new subscription or removed by
    /// the worker.
    ///
    /// [`ProductId`]: crate::ProductId
    // TODO: have the worker drop unsubscribed entries, and re-decode a replaced entry even when its inputs are
    // unchanged, since the new subscription has no snapshot.
    products: Mutex<HashMap<String, Box<dyn Product>>>,

    /// Wakes the worker to poll after a subscribe.
    ///
    /// It holds at most one permit, so several subscribes before the worker next waits wake it once.
    pub(crate) wake: Notify,
}

impl Shared {
    pub(crate) fn new() -> Self {
        Self {
            client_id: Uuid::new_v4().to_string(),
            products: Mutex::new(HashMap::new()),
            wake: Notify::new(),
        }
    }

    /// Registers a subscription to `product` decoded by `P`, replacing an entry with no live subscriptions.
    pub(crate) fn subscribe<P: ProductDecoder>(&self, product: &str) -> Result<Subscription<P::Snapshot, P::Error>> {
        let mut products = self.products.lock().unwrap();
        if products.get(product).is_some_and(|existing| existing.is_subscribed()) {
            return Err(Error::AlreadySubscribed {
                product: product.to_owned(),
            });
        }

        let (publisher, subscription) = Publisher::new();
        let registration = Registration::<P> {
            publisher,
            decoder: PhantomData,
        };
        products.insert(product.to_owned(), Box::new(registration));
        drop(products);

        self.wake.notify_one();
        Ok(subscription)
    }

    /// Decodes `assignment` with the product's decoder and publishes the outcome to its subscriptions.
    ///
    /// Returns each configuration's rejection, if any, or `None` when the product is not registered.
    // TODO: remove dead_code guard when the worker delivers polled assignments.
    #[allow(dead_code)]
    pub(crate) fn assign(
        &self, product: &str, assignment: Vec<(ConfigId, &[u8])>,
    ) -> Option<Vec<(ConfigId, Option<String>)>> {
        let products = self.products.lock().unwrap();
        products
            .get(product)
            .map(|registration| registration.assign(assignment))
    }
}

/// A subscribed product with its decoder type erased, so that products with different decoders share one registry.
trait Product: Send {
    /// Returns whether any clone of the product's subscription is still alive.
    fn is_subscribed(&self) -> bool;

    /// Decodes and publishes an assignment, returning each configuration's rejection, if any.
    // TODO: remove dead_code guard when the worker delivers polled assignments.
    #[allow(dead_code)]
    fn assign(&self, assignment: Vec<(ConfigId, &[u8])>) -> Vec<(ConfigId, Option<String>)>;
}

struct Registration<P: ProductDecoder> {
    publisher: Publisher<P::Snapshot, P::Error>,
    decoder: PhantomData<fn() -> P>,
}

impl<P: ProductDecoder> Product for Registration<P> {
    fn is_subscribed(&self) -> bool {
        self.publisher.is_subscribed()
    }

    fn assign(&self, assignment: Vec<(ConfigId, &[u8])>) -> Vec<(ConfigId, Option<String>)> {
        self.publisher.assign::<P>(assignment)
    }
}
