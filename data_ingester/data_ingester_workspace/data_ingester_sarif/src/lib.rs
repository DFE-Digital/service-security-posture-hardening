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

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let mut hec_events = Vec::new();
        for run in &self.inner.runs {
            let tool = Tool::from(run);

            let results = if let Some(results) = run.results.as_ref() {
                results
            } else {
                continue;
            };
            for result in results {
                
                let rule = result.rule_id.as_ref().and_then(|rule_id| tool.rules.get(rule_id)).cloned();
                
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

impl From<&Vec<ReportingDescriptor>> for Tool {
    fn from(value: &Vec<ReportingDescriptor>) -> Self {
        let mut rules = HashMap::new();
        for rule in value {
            let _ = rules.insert(rule.id.to_string(), rule.clone());
        }
        Tool { rules }
    }
}

impl From<&Run> for Tool {
    fn from(value: &Run) -> Self {
        let mut rules: HashMap<String, ReportingDescriptor> = HashMap::new();

        if let Some(driver_rules) = value.tool.driver.rules.as_ref() {
            for rule in driver_rules {
                dbg!(&rule);
                let _ = rules.insert(rule.id.to_string(), rule.clone());
            }
        }

        if let Some(extensions) = value.tool.extensions.as_ref() {
            for extension in extensions {
                if let Some(extension_rules) = extension.rules.as_ref() {
                    for rule in extension_rules {
                        dbg!(&rule);
                        let _ = rules.insert(rule.id.to_string(), rule.clone());
                    }
                }
            }
        }

        Tool { rules }
    }
}

#[derive(Debug, Serialize)]
pub struct SarifResultSerialize {
    result: SarifResult,
    rule: Option<ReportingDescriptor>,
}

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
    use data_ingester_splunk::splunk::{set_ssphp_run, Splunk};
    use data_ingester_supporting::keyvault::get_keyvault_secrets;

    use super::*;

    pub async fn setup() -> Result<Splunk> {
        let secrets = get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?;
        set_ssphp_run("default")?;
        let splunk = Splunk::new(
            secrets.splunk_host.as_ref().context("No value")?,
            secrets.splunk_token.as_ref().context("No value")?,
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
        assert!(false);
        Ok(())
    }
}

// async fn load_codeql_sarif(splunk: Splunk) -> Result<()> {
//     // let contents =
//     //     fs::read_to_string("codeql.sarif").expect("Should have been able to read the file");
//     // let sarif: Sarif = serde_json::from_str(&contents)?;
//     // dbg!(&sarif.inner.properties);
//     // dbg!(&sarif.inner.inline_external_properties);
//     // dbg!(&sarif.inner.runs.len());
//     // dbg!(&sarif.inner.runs);
//     //    dbg!(&sarif);
//     // dbg!(&sarif.runs[0].results.as_ref().unwrap()[0]);
//     for run in &self.inner.runs {
//         //let tool_rules = run.tool.driver.rules.as_ref().context("No tool in run")?;
//         let tool = Tool::from(run);
//         dbg!(&tool);
//         //dbg!(&run.results);
//         let results = if let Some(results) = run.results.as_ref() {
//             results
//         } else {
//             continue;
//         };
//         for result in results {
//             // dbg!(&result.properties);
//             let for_hec = SarifHecEvent {
//                 inner: SarifResultSerialize {
//                     result: result.clone(),
//                     rule: tool
//                         .rules
//                         .get(dbg!(result.rule_id.as_ref().unwrap()))
//                         .unwrap()
//                         .clone(),
//                 },
//                 source: "codeql_sarif_test".to_string(),
//                 sourcetype: "codeql_sarif_test".to_string(),
//                 ssphp_run_key: "codeql_sarif_test".to_string(),
//             };
//             let hec_events = for_hec.to_hec_events()?;
//             //dbg!(&hec_events);
//             splunk.send_batch(hec_events).await?;
//         }
//     }
//     Ok(())
// }

// async fn load_semgrep_sarif(splunk: Splunk) -> Result<()> {
//     let contents =
//         fs::read_to_string("semgrep.sarif").expect("Should have been able to read the file");
//     let sarif: Sarif = serde_json::from_str(&contents)?;
//     dbg!(&sarif.inner.properties);
//     dbg!(&sarif.inner.inline_external_properties);
//     dbg!(&sarif.inner.runs.len());
//     dbg!(&sarif.inner.runs[0].results.as_ref().unwrap()[0]);
//     for run in &sarif.inner.runs {
//         let results = if let Some(results) = run.results.as_ref() {
//             results
//         } else {
//             continue;
//         };
//         let tool_rules = run.tool.driver.rules.as_ref().context("No tool in run")?;
//         let tool = Tool::from(tool_rules);
//         for result in results {
//             dbg!(&result.properties);
//             let for_hec = SarifHecEvent {
//                 inner: SarifResultSerialize {
//                     result: result.clone(),
//                     rule: tool
//                         .rules
//                         .get(result.rule_id.as_ref().unwrap())
//                         .unwrap()
//                         .clone(),
//                 },
//                 source: "semgrep_sarif_test".to_string(),
//                 sourcetype: "semgrep_sarif_test".to_string(),
//                 ssphp_run_key: "semgrep_sarif_test".to_string(),
//             };
//             let hec_events = for_hec.to_hec_events()?;
//             dbg!(&hec_events);
//             splunk.send_batch(hec_events).await?;
//         }
//     }
//     Ok(())
// }
