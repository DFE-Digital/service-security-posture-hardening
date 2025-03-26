use data_ingester_splunk::splunk::ToHecEvents;
use jiff::Timestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::ado_metadata::AdoMetadata;
use crate::ado_metadata::AdoMetadataTrait;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepoMostRecentCommit {
    metadata: AdoMetadata,
    // organization: String,
    // project_id: String,
    // repositor_id: String,
    commit_date: String,
    stat: Stat,
}

impl ToHecEvents for RepoMostRecentCommit {
    type Item = Self;

    fn source(&self) -> &str {
        &self.metadata.source
    }

    fn sourcetype(&self) -> &str {
        "ADO"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }

    fn ssphp_run_key(&self) -> &str {
        crate::SSPHP_RUN_KEY
    }
}

pub(crate) struct Stats {
    stats: Vec<Stat>,
    metadata: AdoMetadata,
}

impl From<(Vec<Stat>, AdoMetadata)> for Stats {
    fn from(value: (Vec<Stat>, AdoMetadata)) -> Self {
        Self {
            stats: value.0,
            metadata: value.1,
        }
    }
}

impl AdoMetadataTrait for Stats {
    fn set_metadata(&mut self, metadata: crate::ado_metadata::AdoMetadata) {
        self.metadata = metadata;
    }

    fn metadata(&self) -> Option<&crate::ado_metadata::AdoMetadata> {
        Some(&self.metadata)
    }
}

impl Stats {
    pub(crate) fn most_recent_stat(&self) -> Option<&Stat> {
        let mut max_stat_index = 0;
        let max_stat_date = Timestamp::default();
        for (index, stat) in self.stats.iter().enumerate() {
            if stat.most_recent_date() > max_stat_date {
                max_stat_index = index;
            }
        }
        self.stats.get(max_stat_index)
    }
}

/// https://learn.microsoft.com/en-us/rest/api/azure/devops/git/stats/list?view=azure-devops-rest-4.1&tabs=HTTP#gitversiontype
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Stat {
    commit: Commit,
    name: String,
    ahead_count: i64,
    behind_count: i64,
    is_base_version: bool,
}

impl Stat {
    /// Pick the most recent_date from the author & committer
    pub(crate) fn most_recent_date(&self) -> Timestamp {
        let author_date = self.commit.author.date();
        let commit_date = self.commit.committer.date();
        if author_date > commit_date {
            author_date
        } else {
            commit_date
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    commit_id: String,
    parents: Option<Vec<String>>,
    tree_id: String,
    author: Author,
    committer: Author,
    comment: String,
    url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author {
    name: String,
    email: String,
    date: String,
}

impl Author {
    /// Convert `.date` to a Jiff::Timestamp
    fn date(&self) -> Timestamp {
        self.date.parse().unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use jiff::Timestamp;

    use crate::ado_metadata::AdoMetadata;

    use super::{Author, Commit, Stat, Stats};

    /// Fixture for stats tests
    fn fixture_stats() -> Stats {
        Stats {
            stats: vec![
                Stat {
                    commit: Commit {
                        commit_id: "commit1".into(),
                        parents: Some(vec![]),
                        tree_id: "tree1".into(),
                        author: Author {
                            name: "author1".into(),
                            email: "email@domain.com".into(),
                            date: "2014-06-30T18:10:55Z".into(),
                        },
                        committer: Author {
                            name: "author1".into(),
                            email: "email@domain.com".into(),
                            date: "2014-05-30T18:10:55Z".into(),
                        },
                        comment: "todo".into(),
                        url: "https://foo.com".into(),
                    },
                    name: "branchname".into(),
                    ahead_count: 1,
                    behind_count: 2,
                    is_base_version: true,
                },
                Stat {
                    commit: Commit {
                        commit_id: "commit2".into(),
                        parents: Some(vec![]),
                        tree_id: "tree2".into(),
                        author: Author {
                            name: "author2".into(),
                            email: "email@domain.com".into(),
                            date: "2014-04-30T18:10:55Z".into(),
                        },
                        committer: Author {
                            name: "author2".into(),
                            email: "email@domain.com".into(),
                            date: "2014-07-30T18:10:55Z".into(),
                        },
                        comment: "todo".into(),
                        url: "https://foo.com".into(),
                    },
                    name: "otherbranch".into(),
                    ahead_count: 1,
                    behind_count: 2,
                    is_base_version: true,
                },
            ],
            metadata: AdoMetadata::default(),
        }
    }

    /// Check that `.date` is parsed into a `Timestamp`
    #[test]
    fn test_author_date_parsing() {
        let author = Author {
            name: "nametest".to_string(),
            email: "email@domain.com".to_string(),
            date: "2014-06-30T18:10:55Z".to_string(),
        };
        let date = author.date();
        let expected: Timestamp = "2014-06-30T18:10:55Z".parse().unwrap();
        assert_eq!(date, expected);
    }

    /// Check Stat::most_recent_date() can select an author date
    #[test]
    fn test_stat_date_is_most_recent_author() {
        let stats = fixture_stats();

        let most_recent = stats.stats[0].most_recent_date();
        let expected: Timestamp = stats.stats[0].commit.author.date.parse().unwrap();
        assert_eq!(most_recent, expected);
    }

    /// Check Stat::most_recent_date() can select an committer date    
    #[test]
    fn test_stat_date_is_most_recent_committer() {
        let stats = fixture_stats();

        let most_recent = stats.stats[1].most_recent_date();
        let expected: Timestamp = stats.stats[1].commit.committer.date.parse().unwrap();
        assert_eq!(most_recent, expected);
    }

    /// Check Stats::most_recent_stat can select the most recent `Stat`
    #[test]
    fn test_stats_most_recent() {
        let stats = fixture_stats();
        let stat = stats.most_recent_stat().unwrap();
        assert_eq!(stat.commit.commit_id, "commit2".to_string());
    }
}
