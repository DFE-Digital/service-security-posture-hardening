use crate::{GithubResponse, GithubResponses};
use data_ingester_splunk::splunk::ToHecEvents;
use serde::{Deserialize, Serialize};

/// https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#list-repository-workflows
#[derive(Serialize, Default, Debug)]
pub(crate) struct Workflows {
    total_count: usize,
    pub(crate) workflows: Vec<Workflow>,
    source: String,
    sourcetype: String,
}

impl ToHecEvents for &Workflows {
    type Item = Workflow;

    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.workflows.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#list-repository-workflows
#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) struct Workflow {
    id: usize,
    node_id: String,
    name: String,
    pub(crate) path: String,
    state: String,
    created_at: String,
    updated_at: String,
    url: String,
    html_url: String,
    badge_url: String,
}

impl TryFrom<&GithubResponses> for Workflows {
    type Error = anyhow::Error;

    fn try_from(value: &GithubResponses) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.inner.is_empty() {
            anyhow::bail!("No artifacts in Github Response");
        }

        let workflows = value
            .inner
            .iter()
            .filter_map(|response| Workflows::try_from(response).ok())
            .flat_map(|workflows| workflows.workflows.into_iter())
            .collect::<Vec<Workflow>>();

        Ok(Self {
            workflows,
            total_count: 0,
            source: value.inner[0].source.to_string(),
            sourcetype: "github".to_string(),
        })
    }
}

/// Convert a `&GitHubResponse` into `Artifacts`
impl TryFrom<&GithubResponse> for Workflows {
    type Error = anyhow::Error;
    fn try_from(value: &GithubResponse) -> Result<Self, Self::Error> {
        let workflows = value
            .into_iter()
            .filter_map(|value| value.get("workflows"))
            .filter_map(|value| value.as_array())
            .flatten()
            .filter_map(|value| serde_json::from_value::<Workflow>(value.clone()).ok())
            .collect::<Vec<Workflow>>();

        Ok(Self {
            total_count: workflows.len(),
            workflows,
            source: value.source().to_string(),
            sourcetype: value.sourcetype().to_string(),
        })
    }
}
