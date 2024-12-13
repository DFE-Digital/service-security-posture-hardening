pub mod fbp_results;
pub mod validator;

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{set_ssphp_run, HecEvent, Splunk, ToHecEvents};
use data_ingester_supporting::keyvault::Secrets;
use futures::TryStreamExt;
use rand::Rng;
use serde::Serialize;
use std::sync::Arc;
use tiberius::{AuthMethod, Client, Config, Query, QueryItem, Row};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

async fn get_contact_details(secrets: Arc<Secrets>) -> Result<Vec<ContactDetails>> {
    let host = secrets
        .mssql_host
        .as_ref()
        .context("No secret for mssql_host")?;
    let port = secrets
        .mssql_port
        .as_ref()
        .context("No secret for mssql_port")?
        .parse()?;
    let db = secrets
        .mssql_db
        .as_ref()
        .context("No secret for mssql_db")?;
    let username = secrets
        .mssql_username
        .as_ref()
        .context("No secret for mssql_username")?;
    let password = secrets
        .mssql_password
        .as_ref()
        .context("No secret for mssql_password")?;

    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.authentication(AuthMethod::sql_server(username, password));
    config.database(db);
    config.encryption(tiberius::EncryptionLevel::Required);

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    let mut client = match Client::connect(config, tcp.compat_write()).await {
        // Connection successful.
        Ok(client) => client,
        // The server wants us to redirect to a different address
        Err(tiberius::error::Error::Routing { host, port }) => {
            let mut config = Config::new();

            config.host(&host);
            config.port(port);
            config.authentication(AuthMethod::sql_server(username, password));
            config.database(db);
            config.encryption(tiberius::EncryptionLevel::Required);

            let tcp = TcpStream::connect(config.get_addr()).await?;
            tcp.set_nodelay(true)?;

            // we should not have more than one redirect, so we'll short-circuit here.
            Client::connect(config, tcp.compat_write()).await?
        }
        Err(e) => Err(e)?,
    };

    let contact_details_query = ContactDetails::query();

    let mut stream = contact_details_query.query(&mut client).await?;
    let mut contact_details = vec![];
    let source = format!("{}:{}", host, db);
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            let contact_details_row: ContactDetails = (row, source.as_str()).into();
            contact_details.push(contact_details_row);
        }
    }
    Ok(contact_details)
}

pub async fn entrypoint(secrets: Arc<Secrets>, splunk: Arc<Splunk>) -> Result<()> {
    set_ssphp_run("fbp")?;

    let contact_details = get_contact_details(secrets)
        .await
        .context("Getting contact details from FBP")?;

    let contact_details_hec: Vec<HecEvent> = contact_details
        .iter()
        .flat_map(|contact_detail| (&contact_detail).to_hec_events())
        .flat_map(|vec| vec.into_iter())
        .collect();

    let _ = splunk
        .send_batch(contact_details_hec)
        .await
        .context("send to splunk");

    Ok(())
}

impl<T: Into<String>> From<(Row, T)> for ContactDetails {
    fn from((value, source): (Row, T)) -> Self {
        let as_string = |id: &str| value.get(id).map(|s: &str| s.to_string());
        ContactDetails {
            id: value.get("id"),
            stakeholder_type: as_string("stakeholder_type"),
            email_address: as_string("email_address"),
            portfolio: as_string("portfolio"),
            service_line: as_string("service_line"),
            product: as_string("product"),
            cost_centre_code: value.get("cost_centre_code"),
            cost_centre_owner: as_string("cost_centre_owner"),
            cost_centre_title: as_string("cost_centre_title"),
            account_code: as_string("account_code"),
            activity_code: value.get("activity_code"),
            source: source.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContactDetails {
    pub id: Option<i32>,
    pub stakeholder_type: Option<String>,
    pub email_address: Option<String>,
    pub portfolio: Option<String>,
    pub service_line: Option<String>,
    pub product: Option<String>,
    pub cost_centre_code: Option<i32>,
    pub cost_centre_owner: Option<String>,
    pub cost_centre_title: Option<String>,
    pub account_code: Option<String>,
    pub activity_code: Option<i32>,
    #[serde(skip_serializing)]
    pub source: String,
}

impl ContactDetails {
    fn query() -> Query<'static> {
        Query::new(
            "
SELECT
ID AS id,
StakeholderType AS stakeholder_type,
EmailAddress AS email_address,
Portfolio AS portfolio,
Service_Line AS service_line,
Product AS product,
Cost_Centre_Code AS cost_centre_code,
Cost_Centre_Owner AS cost_centre_owner,
Cost_Centre_Title AS cost_centre_title,
Account_Code AS account_code,
Activity_Code AS activity_code
FROM (
    SELECT
        ApplicationProviderEmailAddr AS EmailAddress,
        mt.ID AS ID,
        Portfolio,
        Service_Line,
        Product,
        Cost_Centre_Code,
        Cost_Centre_Owner,
        Cost_Centre_Title,
        Account_Code,
        Activity_Code,
        StakeholderType='hosting_provider'
    FROM dbo.master_tags mt 
    LEFT JOIN dbo.application_provider ap ON mt.ID = ap.ProductID

    UNION ALL

    SELECT
        TechnicalContractEmailAddr,
        mt.ID AS ID,
        Portfolio,
        Service_Line,
        Product,
        Cost_Centre_Code,
        Cost_Centre_Owner,
        Cost_Centre_Title,
        Account_Code,
        Activity_Code,
        StakeholderType='technical_contact'
    FROM dbo.master_tags mt
    LEFT JOIN dbo.technical_contacts tc ON mt.ID = tc.ProductID

    UNION ALL

    SELECT
        HostingProviderEmailAddr,
        mt.ID AS ID,
        Portfolio,
        Service_Line,
        Product,
        Cost_Centre_Code,
        Cost_Centre_Owner,
        Cost_Centre_Title,
        Account_Code,
        Activity_Code,
        StakeholderType='hosting_support'
    FROM dbo.master_tags mt
    LEFT JOIN dbo.hosting_provider hp ON mt.ID = hp.ProductID

    UNION ALL

    SELECT
        ProductOwnerEmailAddr,
        mt.ID AS ID,
        Portfolio,
        Service_Line,
        Product,
        Cost_Centre_Code,
        Cost_Centre_Owner,
        Cost_Centre_Title,
        Account_Code,
        Activity_Code,
        StakeholderType='product_owner'
    FROM dbo.master_tags mt
    LEFT JOIN dbo.product_owner po ON mt.ID = po.ProductID

    UNION ALL

    SELECT
        FBPEmailAddr,
        mt.ID AS ID,
        Portfolio,
        Service_Line,
        Product,
        Cost_Centre_Code,
        Cost_Centre_Owner,
        Cost_Centre_Title,
        Account_Code,
        Activity_Code,
        StakeholderType='financial_business_partner'
    FROM dbo.master_tags mt
    INNER JOIN dbo.fbp fbp ON mt.ID = fbp.ProductID
) 
AS a
ORDER BY a.ID
",
        )
    }

    pub fn generate_contact_details(amount: usize) -> Vec<Self> {
        let mut rng = rand::thread_rng();
        let contact_details: Vec<ContactDetails> = (0..amount)
            .map(|_| Self {
                id: Some(rng.gen()),
                stakeholder_type: Some(format!("Stakeholder Type {}", rng.gen::<i32>() % 5)),
                email_address: Some(format!("email{}@example.com", rng.gen::<i32>() % 100)),
                portfolio: Some(format!("Portfolio {}", rng.gen::<i32>() % 10)),
                service_line: Some(format!("Service Line {}", rng.gen::<i32>() % 5)),
                product: Some(format!("Product {}", rng.gen::<i32>() % 10)),
                cost_centre_code: Some(rng.gen()),
                cost_centre_owner: Some(format!("Cost Centre Owner {}", rng.gen::<i32>() % 5)),
                cost_centre_title: Some(format!("Cost Centre Title {}", rng.gen::<i32>() % 10)),
                account_code: Some(format!("Account Code {}", rng.gen::<i32>() % 100)),
                activity_code: Some(rng.gen()),
                source: "fake_data".into(),
            })
            .collect();
        contact_details
    }
}

impl ToHecEvents for &ContactDetails {
    type Item = ContactDetails;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        "financial_business_partners"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(*self))
    }

    fn ssphp_run_key(&self) -> &str {
        "fbp"
    }
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod live_tests {
    use anyhow::{Context, Result};
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use std::io::Write;
    use std::{collections::HashSet, env, fs::File, sync::Arc};

    #[tokio::test]
    async fn test_entrypoint() -> anyhow::Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();
        set_ssphp_run("fbp")?;

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        super::entrypoint(Arc::new(secrets), Arc::new(splunk))
            .await
            .expect("entrypoint to complete");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn write_contact_details() -> Result<()> {
        let secrets = get_keyvault_secrets(
            &env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
        .await
        .unwrap();

        let contact_details = super::get_contact_details(Arc::new(secrets)).await?;
        let mut set = HashSet::new();

        for contact_detail in contact_details {
            if let Some(service_line) = contact_detail.service_line {
                let _ = set.insert(service_line);
            }
        }
        for value in &set {
            if !value.is_ascii() {
                println!("'''{:?}'''", value);
            }
        }

        let mut file = File::create("service_line.debug")?;
        file.write_all(format!("{:?}", &set).as_bytes())?;

        let json = serde_json::to_string_pretty(&set)?;
        let mut file = File::create("service_line.json")?;
        file.write_all(json.as_bytes())?;

        assert!(false);

        Ok(())
    }
}
