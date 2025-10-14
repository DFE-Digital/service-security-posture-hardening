// use crate::GithubResponses;
// use serde::{Deserialize, Serialize};

// https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#list-repository-workflows
// #[derive(Deserialize, Serialize, Default, Debug)]
// pub(crate) struct WorkflowRunIds {
//     workflow_runs: Vec<i64>,
// }

// /// Convert a `&GitHubResponse` into `Artifacts`
// impl TryFrom<&GithubResponses> for WorkflowRunIds {
//     type Error = anyhow::Error;
//     fn try_from(value: &GithubResponses) -> Result<Self, Self::Error> {
//         let workflow_runs = value
//             .into_iter()
//             .flat_map(|value| value.into_iter())
//             .filter_map(|value| value.get("workflow_runs"))
//             .filter_map(|value| value.as_array())
//             .flatten()
//             .filter_map(|value| value.get("id"))
//             .filter_map(|value| value.as_i64())
//             .collect::<Vec<i64>>();

//         Ok(Self { workflow_runs })
//     }
// }
