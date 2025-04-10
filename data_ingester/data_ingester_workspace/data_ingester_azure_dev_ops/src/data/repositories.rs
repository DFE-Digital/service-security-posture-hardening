use crate::ado_metadata::AdoMetadata;
use crate::ado_response::AdoResponse;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;

use super::stats::Stat;
use super::stats::Stats;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repositories {
    pub repositories: Vec<Repository>,
    pub metadata: AdoMetadata,
}

impl Repositories {
    pub fn iter_active(&self) -> impl Iterator<Item = &Repository> {
        self.repositories.iter().filter(|repo| repo.is_active())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repository {
    default_branch: Option<String>,
    pub(crate) id: String,
    is_disabled: bool,
    is_in_maintenance: bool,
    pub(crate) name: String,
    project: Project,
    remote_url: String,
    size: Option<usize>,
    ssh_url: String,
    url: String,
    web_url: String,

    // Maybe remove if not used
    #[serde(default)]
    most_recent_stat: Option<Stat>,
    // Maybe remove if not used
    #[serde(default)]
    most_recent_commit_date: Option<String>,
    #[serde(default)]
    days_since_last_commit: Option<i64>,
}

impl Repository {
    #[allow(unused)]
    pub fn new(name: String, id: String) -> Self {
        Self {
            name,
            id,
            ..Default::default()
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    #[allow(unused)]
    pub fn is_disabled(&self) -> bool {
        self.is_disabled
    }

    #[allow(unused)]
    pub fn is_in_maintenance(&self) -> bool {
        self.is_in_maintenance
    }

    pub fn is_active(&self) -> bool {
        !self.is_in_maintenance && !self.is_disabled
    }

    pub fn project_id(&self) -> &str {
        self.project.id.as_str()
    }

    pub fn add_most_recent_stat(&mut self, stats: Stats) {
        self.most_recent_stat = stats.most_recent_stat().cloned();
        if let Some(stat) = self.most_recent_stat.as_ref() {
            self.most_recent_commit_date = Some(stat.most_recent_date().to_string());
            let duration = jiff::Timestamp::now()
                .duration_since(stat.most_recent_date())
                .as_hours()
                / 24;
            self.days_since_last_commit = Some(duration);
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Project {
    id: String,
    last_update_time: String,
    name: String,
    revision: usize,
    state: String,
    url: String,
    visibility: String,
}

impl From<AdoResponse> for Repositories {
    fn from(value: AdoResponse) -> Self {
        let repositories = value.value.into_iter().filter_map(|repo| {
            match serde_json::from_value::<Repository>(repo) {
                Ok(repo) => {
                    Some(repo)
                },
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResponse> for Repositories", error=?err);
                    None
                }
            }
        }).collect();
        Self {
            metadata: value.metadata,
            repositories,
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::test_utils::TEST_SETUP;
    use crate::{ado_response::AdoResponse, data::repositories::Repositories};

    use anyhow::Result;

    static REPOSITORIES_JSON: &str = r#"
{
  "count": 1,
  "value": [
    {
      "defaultBranch": "refs/heads/main",
      "id": "8c736a7b-66fb-4d1f-a1c0-de2a6a656d00",
      "isDisabled": false,
      "isInMaintenance": false,
      "name": "bar",
      "project": {
        "id": "2da91f47-0790-47a0-98cc-175fe8fb561e",
        "lastUpdateTime": "2025-01-15T00:58:39.313Z",
        "name": "foo",
        "revision": 44,
        "state": "wellFormed",
        "url": "https://dev.azure.com/aktest0831/_apis/projects/2da91f47-0790-47a0-98cc-175fe8fb561e",
        "visibility": "public"
      },
      "remoteUrl": "https://aktest0831@dev.azure.com/aktest0831/foo/_git/bar",
      "size": 0,
      "sshUrl": "git@ssh.dev.azure.com:v3/aktest0831/foo/bar",
      "url": "https://dev.azure.com/aktest0831/2da91f47-0790-47a0-98cc-175fe8fb561e/_apis/git/repositories/8c736a7b-66fb-4d1f-a1c0-de2a6a656d00",
      "webUrl": "https://dev.azure.com/aktest0831/foo/_git/bar"
    }
  ]
}
"#;

    #[allow(dead_code)]
    pub(crate) fn repopsitory_test_fixture() -> Repositories {
        let ado_response: AdoResponse = serde_json::from_str(REPOSITORIES_JSON).unwrap();
        let repositories: Repositories = Repositories::from(ado_response);
        repositories
    }

    #[test]
    fn test_ado_repositories_json_into_ado_response() {
        let ado_response: AdoResponse = serde_json::from_str(REPOSITORIES_JSON).unwrap();
        assert_eq!(ado_response.count, ado_response.value.len());
    }

    #[test]
    fn test_ado_repositories_from_ado_response() {
        let ado_response: AdoResponse = serde_json::from_str(REPOSITORIES_JSON).unwrap();
        let repositories: Repositories = Repositories::from(ado_response);
        assert_eq!(repositories.repositories.len(), 1);
    }

    #[test]
    fn live_test_repositories_from_ado_response() {
        use crate::{
            ado_dev_ops_client::AzureDevOpsClientMethods, data::repositories::Repositories,
        };

        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let repositories = t
                .ado
                .git_repository_list(&t.organization, &t.project)
                .await?;

            let repositories_len = repositories.value.len();
            assert!(repositories_len > 0);

            let repositories: Repositories = repositories.into();
            assert_eq!(repositories.repositories.len(), repositories_len);
            Ok(())
        });
    }
}
