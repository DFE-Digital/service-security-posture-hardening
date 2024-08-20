use crate::{GithubResponse, GithubResponses};
use anyhow::{anyhow, Result};
use data_ingester_splunk::splunk::ToHecEvents;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use itertools::Itertools;

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

#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) struct WorkflowRuns {
    pub(crate) workflow_runs: Vec<WorkflowRun>,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    pub(crate) total_count: usize,
}

impl ToHecEvents for &WorkflowRuns {
    type Item = WorkflowRun;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.workflow_runs.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

impl TryFrom<&GithubResponses> for WorkflowRuns {
    type Error = anyhow::Error;

    fn try_from(value: &GithubResponses) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.inner.is_empty() {
            anyhow::bail!("No artifacts in Github Response");
        }

        let workflow_runs = value
            .inner
            .iter()
            .filter_map(|response| WorkflowRuns::try_from(response).ok())
            .flat_map(|workflow_runs| workflow_runs.workflow_runs.into_iter())
            .collect::<Vec<WorkflowRun>>();

        Ok(Self {
            total_count: workflow_runs.len(),
            workflow_runs,
            source: value.inner[0].source.to_string(),
            sourcetype: "github".to_string(),
        })
    }
}

/// Convert a `&GitHubResponse` into `Artifacts`
impl TryFrom<&GithubResponse> for WorkflowRuns {
    type Error = anyhow::Error;
    fn try_from(value: &GithubResponse) -> Result<Self, Self::Error> {
        let workflow_runs = value
            .into_iter()
            .filter_map(|value| value.get("workflow_runs"))
            .filter_map(|value| value.as_array())
            .flatten()
            .filter_map(|value| serde_json::from_value::<WorkflowRun>(value.clone()).ok())
            .collect::<Vec<WorkflowRun>>();

        Ok(Self {
            total_count: workflow_runs.len(),
            workflow_runs,
            source: value.source().to_string(),
            sourcetype: value.sourcetype().to_string(),
        })
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) struct WorkflowRun {
    pub(crate) id: i64,
    pub(crate) run_attempt: i64,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub(crate) struct WorkflowRunJobs {
    pub(crate) jobs: Vec<Value>,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    pub(crate) total_count: usize,
}

impl ToHecEvents for &WorkflowRunJobs {
    type Item = Value;

    fn source(&self) -> &str {
        unimplemented!("Use the source from the child Content")
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.jobs.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }

    fn to_hec_events(&self) -> Result<Vec<data_ingester_splunk::splunk::HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|u| {
                let source = u
                    .get("url")
                    .and_then(|url| {
                        url.as_str()
                            .and_then(|s| s.split("https://api.github.com").last())
                    })
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

impl TryFrom<&GithubResponses> for WorkflowRunJobs {
    type Error = anyhow::Error;

    fn try_from(value: &GithubResponses) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.inner.is_empty() {
            anyhow::bail!("No artifacts in Github Response");
        }

        let jobs = value
            .inner
            .iter()
            .filter_map(|response| WorkflowRunJobs::try_from(response).ok())
            .flat_map(|workflow_run_jobs| workflow_run_jobs.jobs.into_iter())
            .collect::<Vec<Value>>();

        Ok(Self {
            total_count: jobs.len(),
            jobs,
            source: value.inner[0].source.to_string(),
            sourcetype: "github".to_string(),
        })
    }
}

/// Convert a `&GitHubResponse` into `Artifacts`
impl TryFrom<&GithubResponse> for WorkflowRunJobs {
    type Error = anyhow::Error;
    fn try_from(value: &GithubResponse) -> Result<Self, Self::Error> {
        let jobs = value
            .into_iter()
            .filter_map(|value| value.get("jobs"))
            .filter_map(|value| value.as_array())
            .flatten()
            //.filter_map(|value| serde_json::from_value::<WorkflowRun>(value.clone()).ok())
            .cloned()
            .collect::<Vec<Value>>();

        Ok(Self {
            total_count: jobs.len(),
            jobs,
            source: value.source().to_string(),
            sourcetype: value.sourcetype().to_string(),
        })
    }
}
