use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Qvs(HashMap<String, Cve>);

impl Qvs {
    pub(crate) fn extend(&mut self, other: Qvs) {
        self.0.extend(other.0)
    }
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cve {
    #[serde(flatten)]
    base: Base,
    #[serde(flatten)]
    contributing_factors: ContributingFactors,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct Base {
    pub id: String,
    pub id_type: String,
    pub qvs: String,
    pub qvs_last_changed_date: usize,
    pub nvd_published_date: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ContributingFactors {
    pub cvss: String,
    pub cvss_version: String,
    pub exploit_maturity: Vec<String>,
    pub threat_actors: Vec<String>,
    pub trending: Vec<usize>,
    pub malware_name: Vec<usize>,
    pub malware_hash: Vec<usize>,
    pub epss: Vec<f64>,
}
