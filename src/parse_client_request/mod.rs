//! Parse the client request and return a Subscription
mod postgres;
mod query;

mod subscription;

pub use self::postgres::PgPool;
// TODO consider whether we can remove `Stream` from public API
pub use subscription::{Stream, Subscription, Timeline};

//#[cfg(test)]
pub use subscription::{Content, Reach};

#[cfg(test)]
mod sse_test;
#[cfg(test)]
mod ws_test;
