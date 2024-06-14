#![feature(async_closure)]
#![feature(lazy_cell)]
pub mod splunk;
mod thread;
mod tracing;
pub use tracing::start_splunk_tracing;
