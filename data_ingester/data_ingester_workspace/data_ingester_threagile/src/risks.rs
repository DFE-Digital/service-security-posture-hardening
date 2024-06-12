use anyhow::Result;
use data_ingester_splunk::splunk::ToHecEvents;
use serde_json::Value;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;


#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(rename_all="snake_case")]
pub(crate) struct RisksJson{
    pub(crate) risks: Vec<Value>,
    pub(crate) service: String,
}

impl RisksJson {
    pub(crate) fn from_file(path: &str, service: &str) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let risks: Vec<Value> = serde_json::from_reader(reader)?;
        Ok(Self {
            risks,
            service: service.into(),
        })
    }
}

impl ToHecEvents for &RisksJson {
    type Item = Value;
    fn source(&self) -> &str {
        self.service.as_str()
    }

    fn sourcetype(&self) -> &str {
        "threagile_risks"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.risks.iter())
    }
}
