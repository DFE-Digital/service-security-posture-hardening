use data_ingester_splunk::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::SSPHP_RUN_KEY;

use super::git_policy_configuration::PolicyConfiguration;
use super::git_policy_configuration::PolicyConfigurations;
use super::projects::Project;
use super::repositories::Repositories;
use super::repositories::Repository;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoPolicyJoins {
    joins: Vec<RepoPolicyJoin>,
    #[serde(skip)]
    source: String,
}

impl ToHecEvents for RepoPolicyJoins {
    type Item = RepoPolicyJoin;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        "ADO"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.joins.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        SSPHP_RUN_KEY
    }
}

impl RepoPolicyJoins {
    pub fn from_repo_policies(
        organization: &str,
        project: &Project,
        repositories: &Repositories,
        policies: &PolicyConfigurations,
    ) -> Self {
        let mut joins = vec![];
        for repo in repositories.repositories.iter() {
            let mut policy_count = 0;

            for policy in &policies.policies {
                if Self::does_policy_apply(repo, policy) {
                    policy_count += 1;
                    let repo_policy = RepoPolicyJoin {
                        organization: organization.to_owned(),
                        project_id: repo.project_id().to_owned(),
                        repo_id: repo.id().to_string(),
                        policy_id: Some(policy.id),
                    };
                    joins.push(repo_policy);
                }
            }

            if policy_count == 0 {
                let repo_policy = RepoPolicyJoin {
                    organization: organization.to_owned(),
                    project_id: repo.project_id().to_owned(),
                    repo_id: repo.id().to_string(),
                    policy_id: None,
                };
                joins.push(repo_policy);
            }
        }
        let source = format!("repo_policy_joins:{}:{}", organization, project.id);
        Self { joins, source }
    }

    fn does_policy_apply(repo: &Repository, policy: &PolicyConfiguration) -> bool {
        let repo_project_id = repo.project_id();
        // Are Repo and Policy are in the Project
        match &policy.project_id {
            Some(policy_project) => {
                if repo_project_id != policy_project {
                    debug!(repo=?repo, "Repo and policy are in different projects");
                    return false;
                }
            }
            None => {
                debug!(policy=?policy, "Policy does NOT have a Project ID");
                return false;
            }
        }

        if policy.settings.scope.is_empty() {
            // What to do when no `scope` exists?
            // No scope / applies to everything?
            debug!("Scope is empty - Policy applies to everything?");
            return true;
        }

        let scope_repo_ids = policy
            .settings
            .scope
            .iter()
            .map(|scope| scope.repository_id.as_ref())
            .collect::<Vec<Option<&String>>>();

        if scope_repo_ids.iter().any(|repo_id| repo_id.is_none()) {
            debug!("Scope.repository_id is None - Policy applies to everything?");
            return true;
        }

        if scope_repo_ids.iter().any(|repo_id| {
            if let Some(repo_id) = repo_id {
                repo.id() == **repo_id
            } else {
                false
            }
        }) {
            debug!("Scope.repository_id matches self.id - Policy applies");
            return true;
        }

        // Policy does not apply
        false
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

#[cfg(test)]
mod test {
    use crate::data::repositories::test::repopsitory_test_fixture;
}
