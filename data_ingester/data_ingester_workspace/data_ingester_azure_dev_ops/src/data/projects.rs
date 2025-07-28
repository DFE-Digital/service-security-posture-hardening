use serde::Deserialize;
use serde::Serialize;
use tracing::error;

use crate::ado_response::AdoResponse;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Projects {
    pub projects: Vec<Project>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub collection: Option<Collection>,
    pub default_team: Option<DefaultTeam>,
    pub default_team_image_url: Option<String>,
    pub description: Option<String>,
    pub id: String,
    pub last_update_time: String,
    pub name: String,
    pub revision: i64,
    pub state: ProjectState,
    pub url: String,
    pub visibility: ProjectVisibility,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectVisibility {
    /// The project is only visible to users with explicit access.
    private,
    /// The project is visible to all.
    #[default]
    public,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectState {
    /// Project is in the process of being deleted.
    deleting,

    /// Project is in the process of being created.
    #[default]
    new,

    /// Project is completely created and ready to use.
    wellFormed,

    /// Project has been queued for creation, but the process has not yet started.
    createPending,

    /// All projects regardless of state except Deleted.
    all,

    /// Project has not been changed.
    unchanged,

    /// Project has been deleted.
    deleted,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub collection_url: String,
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultTeam {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl From<AdoResponse> for Projects {
    fn from(value: AdoResponse) -> Self {
        let projects = value.value.into_iter().filter_map(|project| {
            match serde_json::from_value(project.clone()) {
                Ok(project) => Some(project),
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResponse> for Projects", error=?err, project=?project);
                    None
                }
            }
        }).collect();
        Self { projects }
    }
}

#[cfg(test)]
mod test {
    use crate::{ado_response::AdoResponse, data::projects::Projects};

    use crate::test_utils::TEST_SETUP;

    use crate::ado_dev_ops_client::AzureDevOpsClientMethods;

    use anyhow::Result;

    static PROJECTS_JSON: &str = r#"
{
  "count": 2,
  "value": [
    {
      "collection": {
        "collectionUrl": "https://dev.azure.com/aktest0831/",
        "id": "71645052-a9cf-4f92-8075-3b018969bf4d",
        "name": "aktest0831",
        "url": "https://dev.azure.com/aktest0831/_apis/projectCollections/71645052-a9cf-4f92-8075-3b018969bf4d"
      },
      "defaultTeam": {
        "id": "991d7856-2d9f-45ad-a913-bedeb0a6d6f8",
        "name": "foo Team",
        "url": "https://dev.azure.com/aktest0831/_apis/projects/2da91f47-0790-47a0-98cc-175fe8fb561e/teams/991d7856-2d9f-45ad-a913-bedeb0a6d6f8"
      },
      "id": "2da91f47-0790-47a0-98cc-175fe8fb561e",
      "lastUpdateTime": "0001-01-01T00:00:00",
      "name": "foo",
      "revision": 31,
      "state": "wellFormed",
      "url": "https://dev.azure.com/aktest0831/_apis/projects/2da91f47-0790-47a0-98cc-175fe8fb561e",
      "visibility": "private"
    },
    {
      "id": "2da91f47-0790-47a0-98cc-175fe8fb561e",
      "lastUpdateTime": "0001-01-01T00:00:00",
      "name": "foobar",
      "revision": 32,
      "state": "wellFormed",
      "url": "https://dev.azure.com/aktest0831/_apis/projects/2da91f47-0790-47a0-98cc-175fe8fb561e",
      "visibility": "private"
    }
  ]
}
"#;

    #[test]
    fn test_ado_projects_json_into_ado_response() {
        let ado_response: AdoResponse = serde_json::from_str(PROJECTS_JSON).unwrap();
        assert_eq!(ado_response.count, ado_response.value.len());
    }

    #[test]
    fn test_ado_projects_from_ado_response() {
        let ado_response: AdoResponse = serde_json::from_str(PROJECTS_JSON).unwrap();
        let projects: Projects = Projects::from(ado_response);
        assert_eq!(projects.projects.len(), 2);
    }

    #[test]
    fn live_test_ado_projects_from_projects() {
        let t = &*TEST_SETUP;
        let _: Result<()> = t.runtime.block_on(async {
            let projects = t.ado.projects_list(&t.organization).await?;

            let projects_len = projects.value.len();
            assert!(projects_len > 0);

            let projects: Projects = projects.into();
            assert_eq!(projects.projects.len(), projects_len);
            Ok(())
        });
    }
}
