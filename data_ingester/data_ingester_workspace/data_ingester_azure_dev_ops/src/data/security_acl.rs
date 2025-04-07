use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

pub struct Acls {
    pub(crate) inner: Vec<Acl>,
    //    pub(crate) metadata: Vec<AdoMetadata>,
}

impl Acls {
    pub(crate) fn all_acl_descriptors(&self) -> HashSet<&str> {
        self.inner
            .iter()
            .flat_map(|acl| acl.aces_dictionary.keys())
            .map(|key| key.as_str())
            .collect::<HashSet<&str>>()
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
    pub extended_info: Option<ExtendedInfo>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedInfo {
    effective_allow: Option<i32>,
    inherited_allow: Option<i32>,
    effective_deny: Option<i32>,
    inherited_deny: Option<i32>,
}

impl Acls {
    #[allow(dead_code)]
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.inner
            .iter_mut()
            .for_each(|acl| acl.prepare_for_splunk());
    }
}

impl Acl {
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.aces_vec = self.aces_dictionary.values().cloned().collect();
    }
}

#[cfg(test)]
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
        assert!(acl.inherit_permissions);
    }
}
