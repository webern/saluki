use snafu::Snafu;

/// An error produced by the remote configuration client.
// TODO: define subscription errors alongside the subscription return type.
#[derive(Debug, Snafu)]
#[snafu(display("Remote configuration client error."))]
#[non_exhaustive]
pub struct Error;

/// A rejection of a product's configuration snapshot.
// TODO: define structured attribution and reasons, including missing payloads and cross-payload failures.
#[derive(Debug, Snafu)]
#[snafu(display("Remote configuration was rejected."))]
#[non_exhaustive]
pub struct ApplyError;

/// A result produced by the remote configuration client.
pub type Result<T> = std::result::Result<T, Error>;
