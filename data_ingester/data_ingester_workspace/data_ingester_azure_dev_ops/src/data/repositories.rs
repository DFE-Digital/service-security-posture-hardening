use serde::Deserialize;
use serde::Serialize;
use tracing::error;

use crate::ado_response::AdoResponse;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repositories {
    pub repositories: Vec<Repository>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repository {
    default_branch: Option<String>,
    id: String,
    is_disabled: bool,
    is_in_maintenance: bool,
    pub(crate) name: String,
    project: Project,
    remote_url: String,
    size: usize,
    ssh_url: String,
    url: String,
    web_url: String,
}

impl Repository {
    pub fn id(&self) -> &str {
        self.id.as_str()
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
            match serde_json::from_value(repo) {
                Ok(repo) => Some(repo),
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResponse> for Repositories", error=?err);
                    None
                }
            }
        }).collect();
        Self { repositories }
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "live_tests")]
    use crate::test_utils::TEST_SETUP;
    use crate::{ado_response::AdoResponse, data::repositories::Repositories};

    #[cfg(feature = "live_tests")]
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

    #[cfg(feature = "live_tests")]
    #[test]
    fn live_test_repositories_from_ado_response() {
        use crate::data::repositories::Repositories;

        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let repositories = t.ado.git_repository_list(&t.organization, "foo").await?;

            let repositories_len = repositories.value.len();
            assert!(repositories_len > 0);

            let repositories: Repositories = repositories.into();
            assert_eq!(repositories.repositories.len(), repositories_len);
            Ok(())
        });
    }
}
