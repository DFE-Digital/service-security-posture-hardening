use data_ingester_splunk::splunk::ToHecEvents;
use serde::ser::SerializeMap;
use serde::Deserialize;
use serde::Serialize;
use serde::__private::ser::FlatMapSerializer;
use serde::ser::Serializer;
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
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cve {
    base: Base,
    contributing_factors: ContributingFactors,
}

/// Flatten `base` and `contributing_factors` only when Serializing
impl Serialize for Cve {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        Serialize::serialize(&&self.base, FlatMapSerializer(&mut state))?;
        Serialize::serialize(&&self.contributing_factors, FlatMapSerializer(&mut state))?;
        SerializeMap::end(state)
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
    pub trending: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub malware_name: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub malware_hash: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub epss: Vec<f64>,
}

#[cfg(test)]
mod test {
    use super::Qvs;
    use anyhow::Result;
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
        let output = serde_json::from_str::<Qvs>(data)?;
        dbg!(&output);
        let result = serde_json::to_string_pretty(&output)?;
        let expected = r#"{
  "CVE-2021-36765": {
    "id": "CVE-2021-36765",
    "idType": "CVE",
    "qvs": "28",
    "qvsLastChangedDate": 1642032000,
    "nvdPublishedDate": 1628086500,
    "cvss": "5",
    "cvssVersion": "v2"
  },
  "CVE-2021-36798": {
    "id": "CVE-2021-36798",
    "idType": "CVE",
    "qvs": "78",
    "qvsLastChangedDate": 1642550400,
    "nvdPublishedDate": 1628514900,
    "cvss": "5",
    "cvssVersion": "v2",
    "exploitMaturity": [
      "poc"
    ]
  }
}"#;
        assert_eq!(expected, result);
        Ok(())
    }
}
