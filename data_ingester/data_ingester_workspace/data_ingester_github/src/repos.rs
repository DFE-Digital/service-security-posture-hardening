use data_ingester_splunk::splunk::ToHecEvents;
use octocrab::models::Repository;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Repos {
    inner: Vec<Repository>,
    source: String,
}

/// New type for Vec<[Repository]> including the source of the repository
impl Repos {
    pub(crate) fn new(repos: Vec<Repository>, org: &str) -> Self {
        Self {
            inner: repos,
            source: format!("github:{}", org),
        }
    }

    pub(crate) fn repos(&self) -> &[Repository] {
        self.inner.as_slice()
    }
}

impl ToHecEvents for &Repos {
    type Item = Repository;
    fn source(&self) -> &str {
        &self.source
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}
