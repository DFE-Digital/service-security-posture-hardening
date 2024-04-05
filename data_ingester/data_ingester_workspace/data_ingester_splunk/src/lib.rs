#![feature(async_closure)]
pub mod splunk;
mod thread;
mod tracing;
pub use tracing::start_splunk_tracing;
