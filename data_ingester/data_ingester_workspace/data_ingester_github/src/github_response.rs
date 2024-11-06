use anyhow::{Context, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing::warn;

/// A collection of API responses from Github
#[derive(Serialize, Debug)]
pub(crate) struct GithubResponses {
    inner: Vec<GithubResponse>,
}

impl GithubResponses {
    pub(crate) fn from_response(github_response: GithubResponse) -> Self {
        Self {
            inner: vec![github_response],
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub(crate) fn responses_value_iter(
        &self,
    ) -> impl Iterator<Item = &serde_json::Value> + use<'_> {
        self.into_iter().flat_map(|response| response.into_iter())
    }

    pub(crate) fn responses_iter(&self) -> impl Iterator<Item = &GithubResponse> + use<'_> {
        self.into_iter()
    }

    pub(crate) fn into_inner(self) -> Vec<GithubResponse> {
        self.inner
    }

    pub(crate) fn extend<T: IntoIterator<Item = GithubResponse>>(&mut self, iter: T) {
        self.inner.extend(iter)
    }

    /// Gets the source for the responses
    pub(crate) fn source(&self) -> String {
        let sources = self
            .inner
            .iter()
            .map(|response| response.source.as_str())
            .dedup()
            .collect::<Vec<&str>>();
        if sources.len() != 1 {
            warn!("more than 1 source detected");
        }
        sources.join(",")
    }
}

impl From<Vec<GithubResponse>> for GithubResponses {
    fn from(value: Vec<GithubResponse>) -> Self {
        Self { inner: value }
    }
}

impl<'a> IntoIterator for &'a GithubResponses {
    type Item = &'a GithubResponse;

    type IntoIter = GithubResponsesIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            current: 0,
            collection: &self.inner,
        }
    }
}

pub(crate) struct GithubResponsesIterator<'a> {
    current: usize,
    collection: &'a [GithubResponse],
}

impl<'a> Iterator for GithubResponsesIterator<'a> {
    type Item = &'a GithubResponse;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.collection.get(self.current);
        if value.is_some() {
            self.current += 1;
        }
        value
    }
}

impl ToHecEvents for &GithubResponses {
    type Item = GithubResponse;

    /// Not used
    fn source(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn sourcetype(&self) -> &str {
        unimplemented!()
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a collection of
    /// [data_ingester_splunk::splunk::HecEvent] for each element in
    /// of a Github response, in a collection of github responses.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        Ok(self
            .inner
            .iter()
            .flat_map(|response| response.to_hec_events())
            .flatten()
            .collect())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// An  API responses from Github
#[derive(Serialize, Debug)]
pub(crate) struct GithubResponse {
    #[serde(flatten)]
    response: SingleOrVec,
    #[serde(skip)]
    source: String,
    ssphp_http_status: u16,
}

impl GithubResponse {
    pub(crate) fn new(response: SingleOrVec, source: String, ssphp_http_status: u16) -> Self {
        Self {
            response,
            source,
            ssphp_http_status,
        }
    }

    pub(crate) fn http_status(&self) -> u16 {
        self.ssphp_http_status
    }
}

impl<'a> IntoIterator for &'a GithubResponse {
    type Item = &'a serde_json::Value;

    type IntoIter = GithubResponseIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match &self.response {
            SingleOrVec::Vec(vec) => Self::IntoIter {
                current: 0,
                collection: vec.iter().collect(),
            },
            SingleOrVec::Single(single) => Self::IntoIter {
                current: 0,
                collection: vec![&single],
            },
        }
    }
}

pub(crate) struct GithubResponseIterator<'a> {
    current: usize,
    collection: Vec<&'a serde_json::Value>,
}

impl<'a> Iterator for GithubResponseIterator<'a> {
    type Item = &'a serde_json::Value;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.collection.get(self.current);
        if value.is_some() {
            self.current += 1;
        }
        value.copied()
    }
}

/// Descriminator type to help [serde::Deserialize] deal with API endpoints that return a '{}' or a '[{}]'
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum SingleOrVec {
    Vec(Vec<serde_json::Value>),
    Single(serde_json::Value),
}

impl ToHecEvents for &GithubResponse {
    type Item = Self;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    /// Not used
    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        unimplemented!()
    }

    /// Create a [data_ingester_splunk::splunk::HecEvent] for each
    /// element of a collection returned by a single GitHub api call.
    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        // TODO FIX THIS
        // Shouldn't have to clone all the values :(
        let data = match &self.response {
            SingleOrVec::Single(single) => vec![single.clone()],
            SingleOrVec::Vec(vec) => vec.to_vec(),
        };

        let (ok, _err): (Vec<_>, Vec<_>) = data
            .iter()
            .map(|event| GithubResponse {
                response: SingleOrVec::Single(event.clone()),
                source: self.source.clone(),
                ssphp_http_status: self.ssphp_http_status,
            })
            .map(|gr| {
                data_ingester_splunk::splunk::HecEvent::new_with_ssphp_run(
                    &gr,
                    self.source(),
                    self.sourcetype(),
                    self.get_ssphp_run(),
                )
            })
            .partition_result();
        Ok(ok)
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// Helper for paginating GitHub resoponses.
///
/// Represents the link to the next page of results for a paginated Github request.
///
/// The link is stored as just the path and query elements of the URI
/// for compatibility with OctoCrab authentication
///
#[derive(Debug)]
pub(crate) struct GithubNextLink {
    next: Option<String>,
}

impl GithubNextLink {
    /// Use the exact url as the next link
    pub(crate) fn from_str(url: impl Into<String>) -> Self {
        Self {
            next: Some(url.into()),
        }
    }

    /// Take a `link` header, as returned  by Github, and create a new [GithubNextLink] from it.
    async fn from_link_str(header: &str) -> Self {
        static CELL: OnceCell<Regex> = OnceCell::const_new();
        let regex = CELL
            .get_or_init(|| async {
                Regex::new(r#"<(?<url>[^>]+)>; rel=\"next\""#).expect("Regex is valid")
            })
            .await;

        let next = regex
            .captures(header)
            .and_then(|cap| cap.name("url").map(|m| m.as_str().to_string()))
            .and_then(|url| http::uri::Uri::from_maybe_shared(url).ok())
            .and_then(|uri| uri.path_and_query().map(|pq| pq.as_str().to_string()));

        Self { next }
    }

    /// Create a next link from a [http::Response] from GitHub API.
    pub(crate) async fn from_response<T>(response: &http::Response<T>) -> Result<Self> {
        let header = if let Some(header) = response.headers().get("link") {
            header
                .to_str()
                .context("Unable to parse GitHub link header")?
        } else {
            return Ok(Self { next: None });
        };
        Ok(Self::from_link_str(header).await)
    }

    pub(crate) fn next(&self) -> Option<&str> {
        self.next.as_deref()
    }
}

#[cfg(test)]
mod test_github_next_link {
    use anyhow::Result;

    use crate::GithubNextLink;
    #[tokio::test]
    async fn test_github_links() -> Result<()> {
        let header = "<https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=2>; rel=\"next\", <https://api.github.com/repositories/123456789/dependabot/alerts?per_page=1&page=5>; rel=\"last\"";

        let next = GithubNextLink::from_link_str(header).await;
        assert!(next.next.is_some());
        assert_eq!(
            next.next.unwrap(),
            "/repositories/123456789/dependabot/alerts?per_page=1&page=2".to_string()
        );
        Ok(())
    }
}
