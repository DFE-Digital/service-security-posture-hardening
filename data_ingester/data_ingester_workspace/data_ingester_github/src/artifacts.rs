use crate::{DateTime, GithubResponse, GithubResponses};
use data_ingester_splunk::splunk::ToHecEvents;
use serde::{Deserialize, Serialize};

/// Collection of Artifacts returned from
/// '/repos/OWNER/REPO/actions/artifacts'
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub(crate) struct Artifacts {
    pub(crate) artifacts: Vec<Artifact>,
    total_count: usize,
    source: String,
    sourcetype: String,
}

impl Artifacts {
    /// Dedup artifacts
    /// Sorts by .name/.created_at then dedups keeping the most recent .created_at entry
    pub fn dedup(&mut self) {
        self.artifacts.sort();
        self.artifacts.dedup_by(|a, b| a.name.eq(&b.name));
    }
}

/// Hec Event descriptor for Artifacts
impl ToHecEvents for &Artifacts {
    type Item = Artifact;

    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        &self.sourcetype
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.artifacts.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// Convert a `&GitHubResponses` into `Artifacts`
impl TryFrom<&GithubResponses> for Artifacts {
    type Error = anyhow::Error;

    fn try_from(value: &GithubResponses) -> std::prelude::v1::Result<Self, Self::Error> {
        if value.inner.is_empty() {
            anyhow::bail!("No artifacts in Github Response");
        }

        let artifacts = value
            .inner
            .iter()
            .filter_map(|response| Artifacts::try_from(response).ok())
            .flat_map(|artifacts| artifacts.artifacts.into_iter())
            .collect::<Vec<Artifact>>();

        Ok(Self {
            artifacts,
            total_count: 0,
            source: value.inner[0].source.to_string(),
            sourcetype: "github".to_string(),
        })
    }
}

/// Convert a `&GitHubResponse` into `Artifacts`
impl TryFrom<&GithubResponse> for Artifacts {
    type Error = anyhow::Error;
    fn try_from(value: &GithubResponse) -> Result<Self, Self::Error> {
        let artifacts = value
            .into_iter()
            .filter_map(|value| value.get("artifacts"))
            .filter_map(|value| value.as_array())
            .flatten()
            .filter_map(|value| serde_json::from_value::<Artifact>(value.clone()).ok())
            .collect::<Vec<Artifact>>();

        Ok(Self {
            total_count: artifacts.len(),
            artifacts,
            source: value.source().to_string(),
            sourcetype: value.sourcetype().to_string(),
        })
    }
}

/// An Artifact object
#[derive(Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct Artifact {
    pub(crate) archive_download_url: String,
    created_at: DateTime,
    expired: bool,
    expires_at: DateTime,
    pub(crate) id: u64,
    pub(crate) name: String,
    node_id: String,
    size_in_bytes: u64,
    updated_at: DateTime,
    url: String,
    workflow_run: WorkflowRun,
}

impl Artifact {
    /// The name of the GitHub Organisation the Artifact belongs to.
    pub(crate) fn org_name(&self) -> Option<&str> {
        self.archive_download_url.split('/').nth(4)
    }
    /// The name of the GitHub Repository the Artifact belongs to.    
    pub(crate) fn repo_name(&self) -> Option<&str> {
        self.archive_download_url.split('/').nth(5)
    }
}

impl PartialOrd for Artifact {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Artifact {
    /// Order by name, created_at(newest first), updated_at(newist first)
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.name.cmp(&other.name) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match other.created_at.0.cmp(&self.created_at.0) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        other.updated_at.0.cmp(&self.updated_at.0)
    }
}

/// The workflow that produced the Artifact
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialOrd, Eq, PartialEq)]
struct WorkflowRun {
    head_branch: String,
    head_repository_id: u64,
    head_sha: String,
    id: u64,
    repository_id: u64,
}

#[cfg(test)]
mod test {
    use serde_json::Value;

    use crate::{GithubResponse, GithubResponses};

    use super::Artifacts;

    /// Artifact JSON as `Value`
    fn artifact_as_value() -> Value {
        let body = r#"{
  "total_count": 2,
  "artifacts": [
    {
      "id": 12345,
      "node_id": "MDg6QXJ0aWZhY3QxNjU2NDc3ODQx",
      "name": "sarif-semgrep",
      "size_in_bytes": 243719,
      "url": "https://api.github.com/repos/testorg/testrepo/actions/artifacts/1656477841",
      "archive_download_url": "https://api.github.com/repos/testorg/testrepo/actions/artifacts/1656477841/zip",
      "expired": false,
      "created_at": "2024-07-01T17:29:07Z",
      "updated_at": "2024-07-01T17:29:07Z",
      "expires_at": "2024-09-29T17:28:36Z",
      "workflow_run": {
        "id": 123456,
        "repository_id": 123,
        "head_repository_id": 123,
        "head_branch": "main",
        "head_sha": "e2612e84742b26c8b47fe3f76dec8335ededbc55"
      }
    },
    {
      "id": 1234,
      "node_id": "MDg6QXJ0aWZhY3QxNjYwNzA2ODAx",
      "name": "sarif-semgrep",
      "size_in_bytes": 243023,
      "url": "https://api.github.com/repos/testorg/testrepo/actions/artifacts/1660706801",
      "archive_download_url": "https://api.github.com/repos/testorg/testrepo/actions/artifacts/1660706801/zip",
      "expired": false,
      "created_at": "2024-07-02T17:28:48Z",
      "updated_at": "2024-07-02T17:28:48Z",
      "expires_at": "2024-09-30T17:28:21Z",
      "workflow_run": {
        "id": 2345,
        "repository_id": 1234,
        "head_repository_id": 12345,
        "head_branch": "main",
        "head_sha": "e2612e84742b26c8b47fe3f76dec8335ededbc55"
      }
    }
  ]
}"#;
        serde_json::from_str(body).expect("Artifact JSON to parse correctly")
    }

    /// `Artifacts` loaded from JSON
    fn artifacts() -> Artifacts {
        let value = artifact_as_value();
        let github_response = GithubResponse {
            response: crate::SingleOrVec::Single(value),
            source: "github_test".to_string(),
            ssphp_http_status: 200,
        };
        Artifacts::try_from(&github_response).expect("Artifacts to Parse from GitHubResponse")
    }

    /// A GitHubResponse should convert info `Artifacts`
    #[test]
    fn artifacts_try_from_github_response() {
        let value = artifact_as_value();
        let github_response = GithubResponse {
            response: crate::SingleOrVec::Single(value),
            source: "github_test".to_string(),
            ssphp_http_status: 200,
        };
        let artifacts =
            Artifacts::try_from(&github_response).expect("Artifacts to Parse from GitHubResponse");
        assert_eq!(artifacts.artifacts.len(), 2);
    }

    /// `GithubResponses` should convert into `Artifacts
    #[test]
    fn artifacts_try_from_github_responses() {
        let value = artifact_as_value();
        let github_response = GithubResponse {
            response: crate::SingleOrVec::Single(value),
            source: "github_test".to_string(),
            ssphp_http_status: 200,
        };
        let github_responses = GithubResponses {
            inner: vec![github_response],
        };
        let artifacts = Artifacts::try_from(&github_responses)
            .expect("Artifacts to Parse from GitHubResponses");
        assert_eq!(artifacts.artifacts.len(), 2);
    }

    /// The sort order of `Artifacts` should be name, created_at(newist first)
    #[test]
    fn artifacts_sort() {
        let mut artifacts = artifacts();
        artifacts.artifacts.sort();
        assert_eq!(artifacts.artifacts[0].created_at.0, "2024-07-02T17:28:48Z");
        assert_eq!(artifacts.artifacts[1].created_at.0, "2024-07-01T17:29:07Z");
    }

    /// When `.dedup`ing `Artifacts` the most recent entry should be kept
    #[test]
    fn artifacts_dedup() {
        let mut artifacts = artifacts();
        artifacts.dedup();
        assert_eq!(artifacts.artifacts.len(), 1);
        assert_eq!(artifacts.artifacts[0].created_at.0, "2024-07-02T17:28:48Z");
    }

    /// The `Artifact` owner`s name
    #[test]
    fn artifacts_org_name() {
        let artifacts = artifacts();
        assert_eq!(artifacts.artifacts[0].org_name(), Some("testorg"));
    }

    /// The `Artifact` respo name
    #[test]
    fn artifacts_repo_name() {
        let artifacts = artifacts();
        assert_eq!(artifacts.artifacts[0].repo_name(), Some("testrepo"));
    }
}
