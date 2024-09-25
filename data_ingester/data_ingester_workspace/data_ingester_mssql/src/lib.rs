use std::env;
use anyhow::Context;
use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::get_keyvault_secrets;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use tiberius::{AuthMethod, Client, Config, Query, QueryItem, Row, FromSql, FromSqlOwned};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[tokio::test]
async fn main() -> anyhow::Result<()> {
    let mut config = Config::new();

    config.host("host");
    config.port(1433);
    config.authentication(AuthMethod::sql_server("username", "password"));
    config.database("db");
    config.encryption(tiberius::EncryptionLevel::Required);

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    let secrets = get_keyvault_secrets(
        &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
    )
        .await
        .unwrap();
    let splunk = Splunk::new(
        secrets.splunk_host.as_ref().context("No value")?,
        secrets.splunk_token.as_ref().context("No value")?,
    )?;


    let mut client = Client::connect(config, tcp.compat_write()).await?;
    let mut select = Query::new("SELECT * FROM table");
    let mut stream = select.query(&mut client).await?;
    let contact_details = vec![];
    while let Some(item) = stream.try_next().await? {
        match item {
            // our first item is the column data always
            QueryItem::Metadata(meta) => {
                dbg!(&meta);
            }
            // ... and from there on from 0..N rows
            QueryItem::Row(row) => {
                let contact_details: ContactDetails = row.into();
                let hec_event = (&contact_details).to_hec_events()?;
            }
        }
    }
    splunk.send_batch(contact_details).await.context(context);


    Ok(())
}

impl<'a> From<Row> for ContactDetails {
    fn from(value: Row) -> Self {
        ContactDetails {
            portfolio: value.get(0).map(|s: &str| s.to_string()),
            service_line: value.get(1).map(|s: &str| s.to_string()),
            product: value.get(2).map(|s: &str| s.to_string()),
            cost_centre_code: value.get(3),
            cost_centre_owner: value.get(4).map(|s: &str| s.to_string()),
            cost_centre_title: value.get(5).map(|s: &str| s.to_string()),
            account_code: value.get(6).map(|s: &str| s.to_string()),
            activity_code: value.get(7),
            technical_contact: value.get(8).map(|s: &str| s.to_string()),
            technical_contact_upn: value.get(9).map(|s: &str| s.to_string()),
            technical_contact_email_address: value.get(10).map(|s: &str| s.to_string()),
            product_owner: value.get(11).map(|s: &str| s.to_string()),
            product_owner_email_address: value.get(12).map(|s: &str| s.to_string()),
            product_owner_upn: value.get(13).map(|s: &str| s.to_string()),
            hosting_provider: value.get(14).map(|s: &str| s.to_string()),
            hosting_provider_upn: value.get(15).map(|s: &str| s.to_string()),
            fbp_upn: value.get(16).map(|s: &str| s.to_string()),
            application_provider: value.get(17).map(|s: &str| s.to_string()),
            application_provider_email_address: value.get(18).map(|s: &str| s.to_string()),
            application_provider_upn: value.get(19).map(|s: &str| s.to_string()),
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct ContactDetails {
    portfolio: Option<String>,
    service_line: Option<String>,
    product: Option<String>,
    cost_centre_code: Option<i32>,
    cost_centre_owner: Option<String>,
    cost_centre_title: Option<String>,
    account_code: Option<String>,
    activity_code: Option<i32>,
    technical_contact: Option<String>,
    technical_contact_upn: Option<String>,
    /// Double check column name
    technical_contact_email_address: Option<String>,
    product_owner: Option<String>,
    product_owner_email_address: Option<String>,
    product_owner_upn: Option<String>,
    hosting_provider: Option<String>,
    hosting_provider_upn: Option<String>,
    fbp_upn: Option<String>,
    application_provider: Option<String>,
    application_provider_email_address: Option<String>,
    application_provider_upn: Option<String>,
}

impl ToHecEvents for &ContactDetails {
    type Item = ContactDetails;

    fn source(&self) -> &str {
        "SOMETHING"
    }

    fn sourcetype(&self) -> &str {
        "financial_business_partners"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(*self))
    }

    fn ssphp_run_key(&self) -> &str {
        "mssql"
    }
}
