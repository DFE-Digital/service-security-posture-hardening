#![feature(iter_collect_into)]
pub mod admin_request_consent_policy;
pub mod conditional_access_policies;
pub mod directory_roles;
pub mod groups;
pub mod ms_graph;
pub mod msgraph_data;
pub mod role_assignment_schedule;
pub mod roles;
pub mod users;
pub static SSPHP_RUN_KEY: &str = "m365";
