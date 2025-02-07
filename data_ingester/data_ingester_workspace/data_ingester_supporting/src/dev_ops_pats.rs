use anyhow::{Context, Result};
use azure_security_keyvault::{
    prelude::{KeyVaultGetSecretResponse, KeyVaultSecretBaseIdentifierRaw},
    SecretClient,
};
use std::collections::{hash_map::Entry, HashMap};
use tracing::error;

use crate::secret_identifier::SecretIdentifier;

pub(crate) async fn azure_dev_ops_pats(
    client: &SecretClient,
    secrets: &[KeyVaultSecretBaseIdentifierRaw],
) -> Vec<AdoDevOpsPat> {
    let mut map: HashMap<String, AdoDevOpsPatBuilder> = HashMap::new();
    for secret in secrets
        .iter()
        .filter(|secret| secret.id.contains("azure-dev-ops"))
    {
        let secret_id = match SecretIdentifier::from_str(&secret.id)
            .with_context(|| format!("Extracting secret details from 'id':'{}'", secret.id))
        {
            Ok(secret_id) => secret_id,
            Err(err) => {
                error!(name="KeyVault", operation="Extract Secret identifiers from ID", secret_id=secret.id, err=?err);
                continue;
            }
        };
        
        let value = match client.get(&secret_id.id).await {
            Ok(value) => value,
            Err(err) => {
                error!(name="KeyVault", operation="Get Ado secret", secret_id=secret.id, err=?err);
                continue;
            }
        };
        
        match secret_id.token_type.as_str() {
            
            "pat" => match map.entry(secret_id.name) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().pat = Some(value);
                }
                Entry::Vacant(vacant_entry) => {
                    let _ = vacant_entry.insert(AdoDevOpsPatBuilder::from_pat(value));
                }
            },
            
            "org" => match map.entry(secret_id.name) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().organization = Some(value);
                }
                Entry::Vacant(vacant_entry) => {
                    let _ = vacant_entry.insert(AdoDevOpsPatBuilder::from_org(value));
                }
            },
            
            _ => {
                error!(name: "KeyVault", operation="Build AzureDevOps Pats", error="Unknown token type", secret_id=secret.id);
                continue;
            }
        };
    }
    map.into_iter()
        .filter_map(|(_name, builder)| {
            match builder.build() {
                Ok(built) => Some(built),
                Err(err) => {
                    error!(name="KeyVault", operation="Building ADO Pat", err=?err);                    
                    None
                }
            }
        })
        .collect()
}

#[derive(Default, Debug)]
struct AdoDevOpsPatBuilder {
    organization: Option<KeyVaultGetSecretResponse>,
    pat: Option<KeyVaultGetSecretResponse>,
}

impl AdoDevOpsPatBuilder {
    fn from_pat(pat: KeyVaultGetSecretResponse)  -> Self {
        Self {
            organization: None,
            pat: Some(pat),
        }
    }

    fn from_org(pat: KeyVaultGetSecretResponse)  -> Self {
        Self {
            organization: None,
            pat: Some(pat),
        }
    }
    
    fn build(self) -> Result<AdoDevOpsPat> {
        if self.organization.is_none() {
            anyhow::bail!("organization is not set: {:?}", self)
        }
        if self.pat.is_none() {
            anyhow::bail!("pat is not set: {:?}", self)
        }
        Ok(AdoDevOpsPat {
            organization: self.organization.expect("Already checked"),
            pat: self.pat.expect("Already checked"),
        })
    }
}

/// Holds two parts of a devops PAT token.
#[derive(Debug)]
pub struct AdoDevOpsPat {
    /// The DevOps organization the PAT can access
    organization: KeyVaultGetSecretResponse,
    /// The actual PAT value
    pat: KeyVaultGetSecretResponse,
}

impl AdoDevOpsPat {
    /// The DevOps organization the PAT can access    
    pub fn organization(&self) -> &str {
        self.organization.value.as_str()
    }
    /// The actual PAT value    
    pub fn pat(&self) -> &str {
        self.pat.value.as_str()
    }
}

#[cfg(test)]
mod test {
    use azure_security_keyvault::prelude::{KeyVaultGetSecretResponse, KeyVaultGetSecretResponseAttributes};
    use time::OffsetDateTime;

    use super::AdoDevOpsPat;

    fn create_adodevopspat() -> AdoDevOpsPat {
        AdoDevOpsPat {
            organization: KeyVaultGetSecretResponse {
                value: "orgorg".into(),
                id: "org_id".into(),
                attributes: KeyVaultGetSecretResponseAttributes {
                    enabled: true,
                    expires_on: None,
                    created_on: OffsetDateTime::now_utc(),
                    updated_on: OffsetDateTime::now_utc(),
                    recovery_level: "something".into(),
                }
            },
            pat: KeyVaultGetSecretResponse {
                value: "patpat".into(),
                id: "pat_id".into(),
                attributes: KeyVaultGetSecretResponseAttributes {
                    enabled: true,
                    expires_on: None,
                    created_on: OffsetDateTime::now_utc(),
                    updated_on: OffsetDateTime::now_utc(),
                    recovery_level: "something".into(),
                }
            },
        }
    }    

    #[test]
    fn test_adodevopspat_organization() {
        let pat = create_adodevopspat();
        assert_eq!(pat.organization(), "orgorg");
    }

    #[test]
    fn test_adodevopspat_pat() {
        let pat = create_adodevopspat();
        assert_eq!(pat.pat(), "patpat");
    }
}
