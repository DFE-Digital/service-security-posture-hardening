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
    pub collection: Collection,
    pub default_team: DefaultTeam,
    pub id: String,
    pub last_update_time: String,
    pub name: String,
    pub revision: i64,
    pub state: String,
    pub url: String,
    pub visibility: String,
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
            match serde_json::from_value(project) {
                Ok(project) => Some(project),
                Err(err) => {
                    error!(name="Azure DevOps", operation="From<AdoResposne> for Projects", error=?err);
                    None
                }
            }
        }).collect();
        Self { projects }
    }
}

#[cfg(test)]
mod test {
    use crate::{data::projects::Projects, test_utils::TEST_SETUP};
    use anyhow::Result;

    #[test]
    fn test_ado_projects_from_projects() {
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
