#![feature(hash_extract_if)]
mod generic_collection;
pub mod splunk;
mod tasks;
mod thread;
mod tracing;
pub use tracing::start_splunk_tracing;
