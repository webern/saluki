/// An error produced by the remote configuration client.
// TODO: make this a useful error type for clients, impl std::error::Error
pub struct Error;

/// A result produced by the remote configuration client.
pub type Result<T> = std::result::Result<T, Error>;
