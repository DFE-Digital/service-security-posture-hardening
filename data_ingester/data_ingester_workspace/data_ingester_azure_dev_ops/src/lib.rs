use anyhow::Result;
use data_ingester_splunk::splunk::ToHecEvents;
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct AzureDevOpsClientManual {
    pub(crate) client: reqwest::Client,
    token: Token,
    api_version: String,
    tenant_id: String,
        
}

#[derive(Debug, Deserialize)]
struct Token {
    token_type: TokenType,
    expires_in: usize,
    ext_expires_in: usize,
    access_token: String,
}

#[derive(Debug, Deserialize)]
enum TokenType {
    Bearer
}

impl AzureDevOpsClientManual {
    async fn new(client_id: &str, client_secret: &str, tenant_id: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
        let params = [("client_id", client_id),
                      ("client_secret", client_secret),
                      ("grant_type", "client_credentials"),
                      // Fixed ADO scope
                      ("scope", "499b84ac-1321-427f-aa17-267ca6975798/.default"),
        ];
        let response = client.post(url)
            .form(&params)
            .send().await?;

        let token = response.json().await.unwrap();

        Ok(Self {
            client,
            api_version: "7.2-preview.1".into(),
            token,
            tenant_id: tenant_id.into()
        })
    }

    async fn projects(&self, organization: &str) -> Result<AdoResponse> {
        let organization = "aktest0831";
        let url = format!("https://dev.azure.com/{organization}/_apis/projects?api-version={}", self.api_version);

        let response = self.client.get(&url).bearer_auth(&self.token.access_token).send().await?;
        
        if !response.status().is_success() {
            dbg!(response.status());
            anyhow::bail!("failed request");
        }

        let ado_metadata = AdoMetadata::new(
            &self.tenant_id, 
            url.as_str(),
            organization,
            response.status().as_u16(),
        );
        
        let ado_response = {
            let mut ado_response = response.json::<AdoResponse>().await?;
            ado_response.metadata = Some(ado_metadata);
            ado_response
        };
            
        dbg!(&ado_response);

        Ok(ado_response)
    }
}

struct AdoRateLimiting {}
    

struct AdoPaging {
    continuationToken: Option<String>,    
}

#[derive(Debug, Deserialize, Serialize)]
struct AdoMetadata {
    url: String,
    organization: String,
    status: u16,
    source: String,
    sourcetype: String,
}

impl AdoMetadata {
    fn new(tenant: &str, url: &str, organization: &str, status: u16) -> Self {
        Self {
            source: format!("{}:{}", tenant, url),            
            url: url.into(),
            organization: organization.into(),
            status,
            sourcetype: "ADO".into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct AdoResponse {
    count: usize,
    value: Vec<Value>,
    #[serde(skip_deserializing, default, flatten)]
    metadata: Option<AdoMetadata>
}

impl ToHecEvents for AdoResponse {
    type Item = Value;

    fn source(&self) -> &str {
        self.metadata.as_ref().unwrap().source.as_str()
    }

    fn sourcetype(&self) -> &str {
        self.metadata.as_ref().unwrap().sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.value.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "azure_devops"
    }
}

#[cfg(test)]
mod test {
    use anyhow::Context;
    use data_ingester_splunk::splunk::{Splunk, ToHecEvents};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;
    use anyhow::Result;
    use crate::AzureDevOpsClientManual;
    
    #[tokio::test]
    async fn test_create_azure_dev_ops_manual() -> Result<()> {
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        let tracing_guard = tracing::subscriber::set_default(subscriber);
        
        let secrets = get_keyvault_secrets(
            &std::env::var("KEY_VAULT_NAME").expect("Need KEY_VAULT_NAME enviornment variable"),
        )
            .await
            .unwrap();
        let ado = AzureDevOpsClientManual::new(
            secrets.azure_client_id.as_ref().context("No Azure Client Id")?,
            secrets.azure_client_secret.as_ref().context("No Azure Client Secret")?,
            secrets.azure_tenant_id.as_ref().context("No Azure Tenant Id")?,                        
        ).await.unwrap();
        let projects = ado.projects("foo").await?;

        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
        )?;

        let ian_splunk = Splunk::new(
            secrets.ian_splunk_host.as_ref().context("No value")?,
            secrets.ian_splunk_token.as_ref().context("No value")?,
        )?;
        

        let hec_events = (&projects).to_hec_events()?;

        dbg!(&hec_events);

        splunk.send_batch(hec_events.clone()).await?;
        ian_splunk.send_batch(hec_events.clone()).await?;        

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }    
}
