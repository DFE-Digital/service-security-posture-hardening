use anyhow::{Context, Result};
use bytes::Bytes;
use data_ingester_splunk::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use serde_sarif::sarif::ReportingDescriptor;
use serde_sarif::sarif::Result as SarifResult;
use serde_sarif::sarif::Run;
use std::collections::HashMap;
use std::io::Cursor;
//use std::io::prelude::*;

/// New type wrapper for serde_sarif::sarif::Sarif
#[derive(Debug, Deserialize)]
pub struct Sarif {
    inner: serde_sarif::sarif::Sarif,
}

/// Wraper for Sarif including Hec metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct SarifHec {
    inner: serde_sarif::sarif::Sarif,
    #[serde(default)]
    source: String,
    #[serde(default)]
    sourcetype: String,
    ssphp_run_key: String,
}

impl Sarif {
    /// Load Sarif files from `Bytes` in `.zip` format
    pub fn from_zip_bytes(value: Bytes) -> Result<Vec<Self>> {
        let reader = Cursor::new(value.as_ref());
        let mut zip = zip::ZipArchive::new(reader)?;
        let mut sarifs = Vec::with_capacity(zip.len());
        let zip_file_names: Vec<String> = zip
            .file_names()
            .filter(|filename| filename.contains(".sarif"))
            .map(|name| name.to_string())
            .collect();
        for filename in zip_file_names {
            let file = zip
                .by_name(&filename)
                .with_context(|| format!("Unable to get file '{}' from zip", filename))?;
            let sarif: serde_sarif::sarif::Sarif =
                serde_json::from_reader(file).context("Unable to parse")?;
            sarifs.push(Sarif { inner: sarif });
        }
        Ok(sarifs)
    }

    /// Prepare a Sarif for transmission to Splunk Hec
    pub fn to_sarif_hec<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        self,
        source: S1,
        sourcetype: S2,
        ssphp_run_key: S3,
    ) -> SarifHec {
        SarifHec {
            inner: self.inner,
            source: source.into(),
            sourcetype: sourcetype.into(),
            ssphp_run_key: ssphp_run_key.into(),
        }
    }
}

impl ToHecEvents for SarifHec {
    type Item = SarifResultSerialize;

    fn source(&self) -> &str {
        unimplemented!()
    }

    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn ssphp_run_key(&self) -> &str {
        unimplemented!()
    }

    /// In order to convert full Sarif result to HEC events we need to
    /// match the result with the Tool`s rule for that result.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let mut hec_events = Vec::new();

        for run in &self.inner.runs {
            let results = if let Some(results) = run.results.as_ref() {
                results
            } else {
                continue;
            };

            let tool = Tool::from(run);

            for result in results {
                let rule = result
                    .rule_id
                    .as_ref()
                    .and_then(|rule_id| tool.rules.get(rule_id))
                    .cloned();

                let for_hec = SarifHecEvent {
                    inner: SarifResultSerialize {
                        result: result.clone(),
                        rule,
                    },
                    source: self.source.to_string(),
                    sourcetype: self.sourcetype.to_string(),
                    ssphp_run_key: self.ssphp_run_key.to_string(),
                };

                let sarif_hec_event = for_hec
                    .to_hec_events()
                    .context("Convert SarifHecEvent to HevEvents")?;

                hec_events.extend(sarif_hec_event);
            }
        }
        Ok(hec_events)
    }
}

/// Container for multiple SarifHecs
#[derive(Debug, Deserialize, Serialize)]
pub struct SarifHecs {
    pub inner: Vec<SarifHec>,
}

impl ToHecEvents for &SarifHecs {
    type Item = SarifHec;

    /// Defer conversion of HEC events to each `SarifHec`
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        Ok(self
            .inner
            .iter()
            .filter_map(|sarifhec| sarifhec.to_hec_events().ok())
            .flatten()
            .collect())
    }

    fn source(&self) -> &str {
        unimplemented!()
    }

    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    fn ssphp_run_key(&self) -> &str {
        unimplemented!()
    }
}

/// The tool used to generate the Sarif
///
/// A map between rules / test cases and their ID
#[derive(Debug)]
struct Tool {
    rules: HashMap<String, ReportingDescriptor>,
}

impl From<&Run> for Tool {
    fn from(value: &Run) -> Self {
        let mut rules: HashMap<String, ReportingDescriptor> = HashMap::new();

        // Semgrep Rules
        if let Some(driver_rules) = value.tool.driver.rules.as_ref() {
            for rule in driver_rules {
                let _ = rules.insert(rule.id.to_string(), rule.clone());
            }
        }

        // CodeQL Rules
        if let Some(extensions) = value.tool.extensions.as_ref() {
            for extension in extensions {
                if let Some(extension_rules) = extension.rules.as_ref() {
                    for rule in extension_rules {
                        let _ = rules.insert(rule.id.to_string(), rule.clone());
                    }
                }
            }
        }

        Tool { rules }
    }
}

/// A struct to hold a Sarif Result with its Rule
#[derive(Debug, Serialize)]
pub struct SarifResultSerialize {
    result: SarifResult,
    rule: Option<ReportingDescriptor>,
}

/// Add Splunk HEC metadata to a SarifResultSerialize
#[derive(Debug, Serialize)]
pub struct SarifHecEvent {
    inner: SarifResultSerialize,
    source: String,
    sourcetype: String,
    ssphp_run_key: String,
}

impl ToHecEvents for SarifHecEvent {
    type Item = SarifResultSerialize;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        self.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(&self.inner))
    }

    fn ssphp_run_key(&self) -> &str {
        self.ssphp_run_key.as_str()
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk, SplunkTrait};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use super::*;

    #[allow(dead_code)]
    pub async fn setup() -> Result<Splunk> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        let _ = set_ssphp_run("default")?;
        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
            true,
        )?;
        Ok(splunk)
    }

    #[tokio::test]
    async fn test_load_semgrep_sarif() -> Result<()> {
        let contents =
            fs::read("test/semgrep-sarif.zip").expect("Should have been able to read the file");
        let bytes = Bytes::from(contents);
        let _ = Sarif::from_zip_bytes(bytes)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_load_codeql_sarif() -> Result<()> {
        let contents =
            fs::read("test/codeql-sarif.zip").expect("Should have been able to read the file");
        let bytes = Bytes::from(contents);
        let _ = Sarif::from_zip_bytes(bytes)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_sarif_to_hec_events() -> Result<()> {
        let contents =
            fs::read("test/codeql-sarif.zip").expect("Should have been able to read the file");
        let bytes = Bytes::from(contents);
        let sarifs = Sarif::from_zip_bytes(bytes)?;
        for sarif in sarifs {
            let sarif_hec = sarif.to_sarif_hec("sourcetest", "sourcetypetest", "ssphp_run_keytest");
            let _hec_events = sarif_hec.to_hec_events()?;
            dbg!(_hec_events);
        }
        Ok(())
    }
}
