use crate::{GithubResponse, GithubResponses};
use anyhow::{anyhow, Context, Result};
use base64::prelude::*;
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// https://docs.github.com/en/rest/repos/contents?apiVersion=2022-11-28#get-repository-content
#[derive(Serialize, Default, Debug)]
pub(crate) struct Contents {
    pub(crate) contents: Vec<Content>,
}

/// https://docs.github.com/en/rest/repos/contents?apiVersion=2022-11-28#get-repository-content
#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) struct Content {
    #[serde(rename = "_links")]
    links: Links,
    content: String,
    download_url: String,
    encoding: String,
    git_url: String,
    html_url: String,
    name: String,
    path: String,
    sha: String,
    size: usize,
    r#type: String,
    url: String,
    content_object: Option<Value>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
struct Links {
    git: String,
    #[serde(rename = "self")]
    _self: String,
    html: String,
}

impl Content {
    /// Content from GitHub is Base64 decoded
    ///
    /// We are after the maching readable version of the data so try
    /// YAML and JSON decoding before adding the data to the Content
    /// object
    fn decode_content(&mut self) -> Result<()> {
        let cleaned = self
            .content
            .chars()
            .filter(|chr| !b" \n\t\r\x0b\x0c".contains(&(*chr as u8)))
            .collect::<String>();

        let decoded_bytes = BASE64_STANDARD
            .decode(cleaned)
            .context("Base64 decode GitHub private key")?;

        let decoded_string = String::from_utf8(decoded_bytes.clone())?;

        // Try and decode as YAML
        let mut decoded_object = serde_yaml::from_slice::<Value>(&decoded_bytes).ok();

        // Try and decode as JSON
        decoded_object = if decoded_object.is_none() {
            serde_json::from_slice(&decoded_bytes).ok()
        } else {
            decoded_object
        };

        self.content = decoded_string;
        self.content_object = decoded_object;
        Ok(())
    }
}

impl ToHecEvents for &Contents {
    type Item = Content;

    fn source(&self) -> &str {
        unimplemented!("Use the source from the child Content")
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.contents.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|u| {
                let source = u
                    .url
                    .split("https://api.github.com")
                    .last()
                    .unwrap_or_default();

                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &u,
                    source,
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

impl TryFrom<&GithubResponses> for Contents {
    type Error = anyhow::Error;

    fn try_from(value: &GithubResponses) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.inner.is_empty() {
            anyhow::bail!("No workflows in Github Response");
        }
        let contents = value
            .inner
            .iter()
            .filter_map(|response| Contents::try_from(response).ok())
            .flat_map(|contents| contents.contents.into_iter())
            .collect::<Vec<Content>>();

        Ok(Self { contents })
    }
}

/// Convert a `&GitHubResponse` into `Content`
impl TryFrom<&GithubResponse> for Contents {
    type Error = anyhow::Error;
    fn try_from(value: &GithubResponse) -> Result<Self, Self::Error> {
        let contents = value
            .into_iter()
            .filter_map(|value| serde_json::from_value::<Content>(value.clone()).ok())
            .inspect(|value| {
                dbg!(value);
            })
            .map(|mut content| {
                let _ = content.decode_content();
                content
            })
            .collect::<Vec<Content>>();
        Ok(Self { contents })
    }
}
