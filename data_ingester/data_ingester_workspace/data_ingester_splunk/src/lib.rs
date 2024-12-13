#![feature(async_closure)]
#![feature(hash_extract_if)]
pub mod splunk;
mod tasks;
mod thread;
mod tracing;
pub use tracing::start_splunk_tracing;
