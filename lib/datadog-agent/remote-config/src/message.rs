use crate::product::ProductId;

pub struct Message<T> {
    pub product_id: ProductId,
    pub operation: Operation<T>,
}

/// When the user receives a callback, this tells us what happened.
// TODO: help, I don't understand the protocol and there's a bunch of shit about paths and crap.
pub enum Operation<T> {
    Update(T),
    Remove,
    Error(crate::Error),
}
