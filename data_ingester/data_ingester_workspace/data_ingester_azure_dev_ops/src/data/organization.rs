use crate::ado_response::AdoMetadata;
use anyhow::{anyhow, Context, Result};
use csv::{ReaderBuilder, Trim};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use tracing::error;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Organizations {
    pub(crate) organizations: Vec<Organization>,
    pub(crate) metadata: AdoMetadata,
}

impl Organizations {
    pub(crate) fn from_csv(csv: &str, metadata: AdoMetadata) -> Self {
        let csv_reader = ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(csv.as_bytes());
        let organizations: Vec<Organization> = csv_reader
            .into_deserialize::<Organization>()
            .filter_map(|deserialize| match deserialize {
                Ok(ok) => Some(ok),
                Err(err) => {
                    error!(name="Azure DevOps", operation="Organizations::from_csv", error=?err);
                    None
                }
            })
            .collect();
        Organizations {
            organizations,
            metadata,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Organization {
    #[serde(rename = "Organization Id")]
    organization_id: String,
    #[serde(rename = "Organization Name")]
    pub(crate) organization_name: String,
    #[serde(rename = "Url")]
    url: String,
    #[serde(rename = "Owner")]
    owner: String,
    #[serde(
        default,
        rename = "Exception Type",
        skip_serializing_if = "Option::is_none"
    )]
    exception_type: Option<String>,
    #[serde(
        default,
        rename = "Error Message",
        skip_serializing_if = "Option::is_none"
    )]
    error_message: Option<String>,
}

impl ToHecEvents for &Organizations {
    type Item = Organization;

    fn source(&self) -> &str {
        self.metadata.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        self.metadata.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.organizations.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "azure_devops"
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|organization| {
                let mut organization = serde_json::to_value(organization)?;
                let ssphp_debug = serde_json::to_value(&self.metadata)?;

                let _ = organization
                    .as_object_mut()
                    .context("Getting Organization as Value Object")?
                    .insert("SSPHP_DEBUG".into(), ssphp_debug);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &organization,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
}

#[cfg(test)]
mod test {
    use super::{AdoMetadata, Organizations};

    #[test]
    fn test_organisations_from_csv() {
        let csv = "Organization Id, Organization Name, Url, Owner, Exception Type, Error Message\r\nd7605fe0-ad26-44d1-9b5d-5850e8e9e1f5, CatsCakes, https://dev.azure.com/CatsCakes/, sam@aksecondad.onmicrosoft.com, , \r\n71645052-a9cf-4f92-8075-3b018969bf4d, aktest0831, https://dev.azure.com/aktest0831/, aktest@aksecondad.onmicrosoft.com, , \r\n";
        let metadata = AdoMetadata::new(
            "Tenant",
            "url",
            None,
            None,
            None,
            200,
            "fn test_organisations_from_csv",
            "no docs",
        );
        let orgs = Organizations::from_csv(csv, metadata);
        assert_eq!(orgs.organizations.len(), 2);
    }
}
