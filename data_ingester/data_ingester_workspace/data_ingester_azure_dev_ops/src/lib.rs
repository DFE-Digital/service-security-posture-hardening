#![feature(type_changing_struct_update)]
mod ado_dev_ops_client;
mod ado_metadata;
mod ado_response;
mod azure_dev_ops_client_oauth;
mod azure_dev_ops_client_pat;
mod data;
pub mod entrypoint;
#[cfg(test)]
mod test_utils;

const SSPHP_RUN_KEY: &str = "azure_devops";
