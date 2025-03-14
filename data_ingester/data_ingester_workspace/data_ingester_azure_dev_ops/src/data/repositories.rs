use data_ingester_splunk::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;

use crate::ado_response::AdoResponse;

use super::git_policy_configuration::PolicyConfiguration;
use super::git_policy_configuration::PolicyConfigurations;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repositories {
    pub repositories: Vec<Repository>,
}


pub struct GenericCollectionToSplunk<T: Serialize> {
    pub(crate) collection: Vec<T>,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    pub(crate) ssphp_run_key: String,
}

impl<T: Serialize> ToHecEvents for &GenericCollectionToSplunk<T> {
    type Item = T;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        &self.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.collection.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        self.ssphp_run_key.as_str()
    }
}

impl<T: Serialize> ToHecEvents for GenericCollectionToSplunk<T> {
    type Item = T;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        &self.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.collection.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        self.ssphp_run_key.as_str()
    }
}


impl Repositories {
    pub fn iter_active(&self) -> impl Iterator<Item = &Repository> {
        self.repositories.iter().filter(|repo| repo.is_active())
    }

    pub fn add_policies(&mut self, organization: &str, policies: &PolicyConfigurations) -> Vec<RepoPolicyJoin>{
        let mut repo_policies = vec![];
        for repo in self.repositories.iter_mut() {
            let mut policy_count = 0;
            for policy in &policies.policies {
                if repo.does_policy_apply(&policy) {
                    policy_count += 1;
                    let repo_policy = RepoPolicyJoin {
                        organization: organization.to_owned(),
                        project_id: repo.project_id().to_owned(),
                        repo_id: repo.id.clone(),
                        policy_id: Some(policy.id),
                    };
                    repo_policies.push(repo_policy);
                }
            }
            if policy_count == 0 {
                let repo_policy = RepoPolicyJoin {
                    organization: organization.to_owned(),
                    project_id: repo.project_id().to_owned(),
                    repo_id: repo.id.clone(),
                    policy_id: None,
                };
                repo_policies.push(repo_policy);

            }
        }
        repo_policies
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoPolicyJoin {
    pub(crate) organization: String,
    pub(crate) project_id: String,
    pub(crate) repo_id: String,
    pub(crate) policy_id: Option<i64>,
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
    size: Option<usize>,
    ssh_url: String,
    url: String,
    web_url: String,
    
    #[serde(default)]    
    policies: Vec<i64>,

}

impl Repository {
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    // pub fn is_disabled(&self) -> bool {
    //     self.is_disabled
    // }

    // pub fn is_in_maintenance(&self) -> bool {
    //     self.is_in_maintenance
    // }

    pub fn is_active(&self) -> bool {
        !self.is_in_maintenance && !self.is_disabled
    }

    pub fn project_id(&self) -> &str {
        self.project.id.as_str()
    }


}

impl PolicyMatch for Repository {
    fn project_id(&self) -> &str {
        self.project_id()
    }

    fn repo_id(&self) -> &str {
        self.id.as_str()
    }
}

trait PolicyMatch: std::fmt::Debug {
    fn project_id(&self) -> &str;
    fn repo_id(&self) -> &str;
    
    fn does_policy_apply(&self, policy: &PolicyConfiguration) -> bool {
        let repo_project_id = self.project_id();
        // Are Repo and Policy are in the Project
        match &policy.project_id {
            Some(policy_project) => {
                if repo_project_id != policy_project {
                    error!(repo=?self, "Repo and policy are in different projects");
                    return false
                }
            },
            None => {
                error!(policy=?policy, "Policy does NOT have a Project ID");
                return false
            },
        }

        if policy.settings.scope.is_empty() {
            // What to do when no `scope` exists?
            // No scope / applies to everything?
            error!("Scope is empty - Policy applies to everything?");
            return true
        }

        let scope_repo_ids = policy.settings.scope.iter().map(|scope| scope.repository_id.as_ref()).collect::<Vec<Option<&String>>>();

        if scope_repo_ids.iter().any(|repo_id| repo_id.is_none()) {
            error!("Scope.repository_id is None - Policy applies to everything?");
            return true
        }

        if scope_repo_ids.iter().any(|repo_id| {
            if let Some(repo_id) = repo_id {
                self.repo_id() == **repo_id
            } else {
                false
            }}) {
            error!("Scope.repository_id matches self.id - Policy applies");                    
            return true
        }

        // Policy does not apply
        false
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
