use data_ingester_splunk::splunk::ToHecEvents;
use serde::ser::Serializer;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Qvs(HashMap<String, Cve>);

/// Data from the Qualys Qvs Endpoint
impl Qvs {
    pub(crate) fn extend(&mut self, other: Qvs) {
        self.0.extend(other.0)
    }
}

impl ToHecEvents for &Qvs {
    type Item = Cve;

    fn source(&self) -> &str {
        "qualys_vulnerability_score"
    }

    fn sourcetype(&self) -> &str {
        "qualys"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.0.values())
    }

    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
    }
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cve {
    base: Base,
    contributing_factors: ContributingFactors,
}

/// Flatten `base` and `contributing_factors` only when Serializing.
/// We delegate to `CveFlat` — a serialize-only DTO whose fields sit at the top
/// level — so we can rely on stable serde APIs instead of reaching into
/// `serde::__private`.
impl Serialize for Cve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CveFlat::from(self).serialize(serializer)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CveFlat<'a> {
    id: &'a str,
    id_type: &'a str,
    qvs: &'a str,
    qvs_last_changed_date: usize,
    nvd_published_date: usize,
    cvss: &'a str,
    cvss_version: &'a str,
    #[serde(skip_serializing_if = "slice_is_empty")]
    exploit_maturity: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    threat_actors: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    trending: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    malware_name: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    malware_hash: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    epss: &'a [String],
}

fn slice_is_empty<T>(v: &&[T]) -> bool {
    v.is_empty()
}

impl<'a> From<&'a Cve> for CveFlat<'a> {
    fn from(cve: &'a Cve) -> Self {
        Self {
            id: &cve.base.id,
            id_type: &cve.base.id_type,
            qvs: &cve.base.qvs,
            qvs_last_changed_date: cve.base.qvs_last_changed_date,
            nvd_published_date: cve.base.nvd_published_date,
            cvss: &cve.contributing_factors.cvss,
            cvss_version: &cve.contributing_factors.cvss_version,
            exploit_maturity: &cve.contributing_factors.exploit_maturity,
            threat_actors: &cve.contributing_factors.threat_actors,
            trending: &cve.contributing_factors.trending,
            malware_name: &cve.contributing_factors.malware_name,
            malware_hash: &cve.contributing_factors.malware_hash,
            epss: &cve.contributing_factors.epss,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    pub id: String,
    pub id_type: String,
    pub qvs: String,
    pub qvs_last_changed_date: usize,
    pub nvd_published_date: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributingFactors {
    pub cvss: String,
    pub cvss_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exploit_maturity: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threat_actors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trending: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub malware_name: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub malware_hash: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub epss: Vec<String>,
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::Qvs;
    use anyhow::Result;
    use serde_json::{json, Value};

    /// Checks that serialization and deserialization use the correct formats
    #[test]
    fn test_qvs_deserialization_serialization() -> Result<()> {
        let data = r#"
{
  "CVE-2021-36765": {
    "base": {
      "id": "CVE-2021-36765",
      "idType": "CVE",
      "qvs": "28",
      "qvsLastChangedDate": 1642032000,
      "nvdPublishedDate": 1628086500
    },
    "contributingFactors": {
      "cvss": "5",
      "cvssVersion": "v2"
    }
  },
  "CVE-2021-36798": {
    "base": {
      "id": "CVE-2021-36798",
      "idType": "CVE",
      "qvs": "78",
      "qvsLastChangedDate": 1642550400,
      "nvdPublishedDate": 1628514900
    },
    "contributingFactors": {
      "cvss": "5",
      "cvssVersion": "v2",
      "exploitMaturity": [
        "poc"
      ]
    }
  }
}"#;
        // Deserialize into QVS
        let output = serde_json::from_str::<Qvs>(data)?;

        // Serialize
        let result = serde_json::to_string_pretty(&output)?;

        // Deserialize into Value
        let qvs = serde_json::from_str::<HashMap<String, Value>>(&result)?;
        for (id, qv) in qvs.iter() {
            match id.as_str() {
                "CVE-2021-36765" => {
                    let expected_data = [
                        ("id", json!("CVE-2021-36765")),
                        ("idType", json!("CVE")),
                        ("qvs", json!("28")),
                        ("qvsLastChangedDate", json!(1642032000)),
                        ("nvdPublishedDate", json!(1628086500)),
                        ("cvss", json!("5")),
                        ("cvssVersion", json!("v2")),
                    ];
                    for (field, value) in expected_data.iter() {
                        assert_eq!(qv.get(field).expect("Field should exist"), value);
                    }
                }
                "CVE-2021-36798" => {
                    let expected_data = [
                        ("id", json!("CVE-2021-36798")),
                        ("idType", json!("CVE")),
                        ("qvs", json!("78")),
                        ("qvsLastChangedDate", json!(1642550400)),
                        ("nvdPublishedDate", json!(1628514900)),
                        ("cvss", json!("5")),
                        ("cvssVersion", json!("v2")),
                        ("exploitMaturity", json!(vec!["poc"])),
                    ];
                    for (field, value) in expected_data.iter() {
                        assert_eq!(qv.get(field).expect("Field should exist"), value);
                    }
                }
                _ => unreachable!("Should be unreachable"),
            }
        }
        Ok(())
    }
}
