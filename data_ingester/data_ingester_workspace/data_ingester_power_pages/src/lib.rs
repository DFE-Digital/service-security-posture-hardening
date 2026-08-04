pub mod client;
pub mod entrypoint;
mod metadata;
mod models;
mod response;
mod summary;

pub use entrypoint::entrypoint;
pub use metadata::{DataType, PowerPagesMetadata};
pub use models::{
    EnvironmentResponse, ScanRule, SiteSecurityResult, SiteSecurityScore, WebsiteDto,
};
pub use response::{CollectionOutcome, PowerPagesApiResult};
pub use summary::RunSummary;

pub static SSPHP_RUN_KEY: &str = "power_pages";
pub const SOURCETYPE: &str = "ssphp:power_pages:json";

#[cfg(test)]
mod tests;

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests;
