use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

use crate::ado_response::AdoResponse;

#[derive(Default)]
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

    pub fn extend(&mut self, other: Acls) {
        self.inner.extend(other.inner)
    }

    #[allow(dead_code)]
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.inner
            .iter_mut()
            .for_each(|acl| acl.prepare_for_splunk());
    }
}

impl From<AdoResponse> for Acls {
    fn from(value: AdoResponse) -> Self {
        let inner = value
            .value
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .map(|mut acl: Acl| {
                acl.prepare_for_splunk();
                acl
            })
            .collect();
        Acls { inner }
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

impl Acl {
    pub(crate) fn prepare_for_splunk(&mut self) {
        self.aces_vec = self.aces_dictionary.values().cloned().collect();
    }
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

#[cfg(test)]
pub(crate) mod test {
    use super::{Acl, Acls};
    use crate::{
        ado_dev_ops_client::AzureDevOpsClientMethods, ado_response::AdoResponse,
        test_utils::TEST_SETUP,
    };
    use anyhow::Result;

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

    fn acl() -> Acl {
        serde_json::from_str(ACL).unwrap()
    }

    fn acls() -> Acls {
        Acls { inner: vec![acl()] }
    }

    pub fn ado_response_for_access_control_lists() -> AdoResponse {
        let t = &*TEST_SETUP;
        let namespaces = crate::data::security_namespaces::test::security_namespace();
        let namespace = namespaces
            .namespaces
            .iter()
            .find(|namespace| namespace.name == "Git Repositories")
            .unwrap();
        let result: Result<AdoResponse> = t.runtime.block_on(async {
            let result = t
                .ado
                .security_access_control_lists(&t.organization, &namespace.namespace_id)
                .await?;
            assert!(!result.value.is_empty());
            Ok(result)
        });
        result.unwrap()
    }

    pub(crate) fn acls_from_ado_response() -> Acls {
        let ado_response = ado_response_for_access_control_lists();
        Acls::from(ado_response)
    }

    #[test]
    fn test_acl_from_json() {
        let acl = acl();
        assert!(acl.inherit_permissions);
    }

    #[test]
    fn acl_prepare_for_splunk() {
        let mut acl = acl();
        assert!(!acl.aces_dictionary.is_empty());
        assert!(acl.aces_vec.is_empty());
        acl.prepare_for_splunk();
        assert_eq!(acl.aces_dictionary.len(), acl.aces_vec.len());
    }

    #[test]
    fn acls_preare_for_splunk() {
        let mut acls = acls();
        acls.prepare_for_splunk();
        assert!(acls.inner.iter().all(|acl| !acl.aces_vec.is_empty()));
    }

    #[test]
    fn acls_all_acl_descriptors() {
        let acls = acls();
        let descriptors = acls.all_acl_descriptors();
        assert!(!descriptors.is_empty());
        assert!(descriptors
            .iter()
            .all(|descriptor| descriptor.contains("Microsoft") && descriptor.len() > 20));
    }

    #[test]
    fn test_acls_from_ado_response() {
        let acls = acls_from_ado_response();
        assert!(!acls.inner.is_empty());
    }

    #[test]
    fn acls_from_ado_response_should_fill_aces_vec() {
        let acls = acls_from_ado_response();
        assert!(acls.inner.iter().all(|acl| !acl.aces_vec.is_empty()));
    }

    #[test]
    fn acls_extend() {
        let mut acls1 = acls();
        let acls2 = acls();
        let acls1_len = acls1.inner.len();
        let acls2_len = acls1.inner.len();
        acls1.extend(acls2);
        assert_eq!(acls1.inner.len(), acls1_len + acls2_len);
    }
}
