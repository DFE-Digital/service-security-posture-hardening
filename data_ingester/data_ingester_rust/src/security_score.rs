use crate::splunk::{HecEvent, ToHecEvents};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScores {
    #[serde(rename = "@odata.context")]
    pub odata_context: String,
    #[serde(rename = "@odata.nextLink")]
    pub odata_next_link: Option<String>,
    #[serde(rename = "value")]
    pub inner: Vec<Value>,
}

impl ToHecEvents for &SecurityScores {
    type Item = Value;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "m365:control_score"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[cfg(test)]
mod test {
    use crate::{ms_graph::test::setup, splunk::ToHecEvents};
    use anyhow::Result;
    #[tokio::test]
    async fn test_to_hec_events_collection() -> Result<()> {
        let (_splunk, ms_graph) = setup().await?;
        let security_scores = ms_graph.get_security_secure_scores().await?;
        let _hec_events = (&security_scores).to_hec_events()?;
        Ok(())
    }
}
