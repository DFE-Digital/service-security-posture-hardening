
use anyhow::{anyhow, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, trace};

use crate::{
    ado_metadata::{AdoMetadata, AdoMetadataTrait},
    SSPHP_RUN_KEY,
};

use std::collections::HashMap;

pub struct Acls {
    pub(crate) acls: Vec<Acl>,
    pub(crate) metadata: Option<AdoMetadata>,
}

impl AdoMetadataTrait for Acls {
    fn set_metadata(&mut self, metadata: AdoMetadata) {
        self.metadata = Some(metadata);
    }

    fn metadata(&self) -> Option<&AdoMetadata> {
        self.metadata.as_ref()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Acl {
    #[serde(skip_serializing, default)]    
    pub aces_dictionary: HashMap<String, AclEntry>,
    #[serde(skip_deserializing, default)]
    pub aces_vec: Vec<AclEntry>,    
    pub inherit_permissions: bool,
    pub token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AclEntry {
    pub allow: i64,
    pub deny: i64,
    pub descriptor: String,
}

impl Acls {
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.acls.iter_mut().for_each(|acl| acl.prepare_for_splunk());
    }
}

impl Acl {
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.aces_vec = self.aces_dictionary.values().cloned().collect();
    }
}

impl ToHecEvents for Acls {
    type Item = Acl;

    fn source(&self) -> &str {
        self.metadata_source()
    }

    fn sourcetype(&self) -> &str {
        self.metadata_sourcetype()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.acls.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|acl| {
                let mut acl = serde_json::to_value(&acl).unwrap();
                let metadata = if let Some(metadata) = &self.metadata {
                    serde_json::to_value(metadata).unwrap_or_else(|_| {
                        serde_json::to_value("Error Getting AdoMetadata")
                            .expect("Value from static str should not fail")
                    })
                } else {
                    serde_json::to_value("No AdoMetadata")
                        .expect("Value from static str should not fail")
                };

                let _ = acl
                    .as_object_mut()
                    .expect("ado_response should always be accessible as an Value object")
                    .insert("metadata".into(), metadata);
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &acl,
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

mod test {
    use super::Acl;
    static ACL: &str = r#"{
  "acesDictionary": {
    "Microsoft.IdentityModel.Claims.ClaimsIdentity;1ed6d920-41e7-46ff-83c2-cd1e713b1c4c\\aktest@aksecondad.onmicrosoft.com": {
      "allow": 3,
      "deny": 0,
      "descriptor": "Microsoft.IdentityModel.Claims.ClaimsIdentity;1ed6d920-41e7-46ff-83c2-cd1e713b1c4c\\aktest@aksecondad.onmicrosoft.com"
    },
    "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1193257261-2416418887-2563512159-3908785694-1-1215788955-3549006664-2706082709-1554640253": {
      "allow": 241,
      "deny": 0,
      "descriptor": "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1193257261-2416418887-2563512159-3908785694-1-1215788955-3549006664-2706082709-1554640253"
    },
    "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1193257261-2416418887-2563512159-3908785694-1-1978387880-3501734468-2782246346-4123287324": {
      "allow": 17,
      "deny": 0,
      "descriptor": "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1193257261-2416418887-2563512159-3908785694-1-1978387880-3501734468-2782246346-4123287324"
    },
    "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1204400969-2402986413-2179408616-0-0-0-4-1": {
      "allow": 17,
      "deny": 0,
      "descriptor": "Microsoft.TeamFoundation.Identity;S-1-9-1551374245-1204400969-2402986413-2179408616-0-0-0-4-1"
    },
    "Microsoft.TeamFoundation.ServiceIdentity;fb169273-e543-4904-a3cd-62d08241ed53:Build:2da91f47-0790-47a0-98cc-175fe8fb561e": {
      "allow": 49,
      "deny": 0,
      "descriptor": "Microsoft.TeamFoundation.ServiceIdentity;fb169273-e543-4904-a3cd-62d08241ed53:Build:2da91f47-0790-47a0-98cc-175fe8fb561e"
    }
  },
  "inheritPermissions": true,
  "token": "vstfs:///Classification/Node/c49533b0-4d02-49cf-a908-f2ef3af5d518"
}"#;

    #[test]
    fn test_acl_from_json() {
        let acl: Acl = serde_json::from_str(ACL).unwrap();
        dbg!(&acl);
        assert_eq!(acl.inherit_permissions, true );
        println!("{}", serde_json::to_string_pretty(&acl).unwrap());
        assert!(false);
    }
}
