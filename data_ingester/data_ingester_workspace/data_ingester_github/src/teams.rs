use std::{collections::HashMap, fmt::Display};

use anyhow::{Context, Result};
use data_ingester_splunk::splunk::{HecEvent, ToHecEvents};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::GithubResponses;

/// Representation of a GitHub org to calculate nested team membership
#[derive(Default, Debug)]
pub(crate) struct GitHubTeamsOrg {
    /// Name of the Org
    name: String,
    /// Teams in the Org
    teams: HashMap<TeamId, Team>,
    /// Members in the Org
    members: HashMap<MemberId, Member>,
}

impl GitHubTeamsOrg {
    /// Create a new Organisation
    ///
    /// `name`: The name of the GitHub Organisation
    ///
    pub(crate) fn new<T: Into<String>>(name: T) -> Self {
        GitHubTeamsOrg {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Get the team members for a specific team
    /// Includes members of all child teams
    fn team_members(&self, id: &TeamId) -> Option<Vec<MemberId>> {
        let team = self
            .team_by_id(id)
            .context(format!("No team with Id {:?}", id))
            .ok()?;
        let mut members = vec![];
        members.extend(team.members());
        for team_id in &team.teams {
            if let Some(child_members) = self.team_members(team_id) {
                members.extend(child_members);
            }
        }
        members.sort();
        members.dedup();
        Some(members)
    }

    /// Return a specific team by ID
    fn team_by_id(&self, id: &TeamId) -> Option<&Team> {
        self.teams.get(id)
    }

    /// Return a Vec of Members for a Vec of MemberIds
    fn members_from_ids(&self, members: &[MemberId]) -> Vec<&Member> {
        members.iter().flat_map(|id| self.members.get(id)).collect()
    }

    /// Add a team to the Organisation
    ///
    /// `team`: a `serde_json::Value` from the GitHub API representating a team.
    ///
    /// https://docs.github.com/en/rest/teams/teams?apiVersion=2022-11-28#list-teams
    ///
    pub(crate) fn push_team_value(&mut self, team: &Value) -> Result<()> {
        let team: Team =
            serde_json::from_value(team.clone()).context("Convert team `Value` to `Team`")?;
        if !self.teams.contains_key(&team.id) {
            let _ = self.teams.insert(team.id.clone(), team);
        }

        Ok(())
    }

    /// Add a collection of members to a team in the Organisation
    ///
    /// `team_id` - The team ID to add members to. The team must already have been added to the org
    /// `responses` A `GithubResponses` object containing responses of team members
    ///
    pub(crate) fn push_team_members_responses(
        &mut self,
        team_id: u64,
        responses: &GithubResponses,
    ) -> Result<()> {
        let team = self
            .teams
            .get_mut(&TeamId(team_id))
            .context("No team with matching ID")?;
        for response in &responses.inner {
            match response.response {
                crate::SingleOrVec::Vec(ref members) => {
                    for member in members {
                        let member = Member::try_from(member.clone())?;
                        team.members.push(member.id.clone());
                        if !self.members.contains_key(&member.id) {
                            let _ = self.members.insert(member.id.clone(), member);
                        }
                    }
                }
                crate::SingleOrVec::Single(ref member) => {
                    let member = Member::try_from(member.clone())?;
                    team.members.push(member.id.clone());
                    if !self.members.contains_key(&member.id) {
                        let _ = self.members.insert(member.id.clone(), member);
                    }
                }
            }
        }
        Ok(())
    }

    /// Add a collection of members to a team in the Organisation
    ///
    /// `team_id` - The team ID to add members to. The team must already have been added to the org
    /// `responses` A `GithubResponses` object containing responses of team members
    ///
    pub(crate) fn push_team_teams_responses(
        &mut self,
        team_id: u64,
        responses: &GithubResponses,
    ) -> Result<()> {
        let team = self
            .teams
            .get_mut(&TeamId(team_id))
            .context("No team with matching ID")?;
        let mut new_teams = vec![];
        for response in &responses.inner {
            match response.response {
                crate::SingleOrVec::Vec(ref child_teams) => {
                    for child_team_value in child_teams {
                        let child_team = Team::try_from(child_team_value.clone())?;
                        team.teams.push(child_team.id.clone());
                        new_teams.push(child_team);
                    }
                }
                crate::SingleOrVec::Single(ref child_team_value) => {
                    let child_team = Team::try_from(child_team_value.clone())?;
                    team.teams.push(child_team.id.clone());
                    new_teams.push(child_team);
                }
            }
        }
        for new_team in new_teams {
            if !self.teams.contains_key(&new_team.id) {
                let _ = self.teams.insert(new_team.id.clone(), new_team);
            }
        }
        Ok(())
    }

    fn serialize_teams(&mut self) -> Result<Vec<TeamSerialize>> {
        let mut teams = vec![];
        for (team_id, team) in self.teams.iter() {
            let member_ids = self
                .team_members(team_id)
                .with_context(|| format!("Getting members for team {}/{}", team_id, team.name))?;
            let members = self
                .members_from_ids(&member_ids)
                .iter()
                .map(|member| MemberSerialize::from(*member))
                .collect();
            let child_teams = team
                .teams
                .iter()
                .flat_map(|team_id| self.team_by_id(team_id))
                .map(|team| ChildTeamSerialize {
                    name: team.name.to_string(),
                    id: team.id.0,
                })
                .collect();
            teams.push(TeamSerialize {
                organisation: self.name.clone(),
                name: team.name.clone(),
                id: team.id.0,
                members,
                teams: child_teams,
                parent: team.parent.clone(),
            });
        }
        Ok(teams)
    }

    pub(crate) fn team_members_hec_events(&mut self) -> Result<Vec<HecEvent>> {
        let teams = self
            .serialize_teams()
            .context("Calculating teams and members")?
            .iter()
            .flat_map(|team| team.to_hec_events())
            .flatten()
            .collect();
        Ok(teams)
    }
}

/// The GitHub ID for a Team
#[derive(Hash, Debug, Default, Deserialize, Serialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
struct TeamId(u64);

impl Display for TeamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A GitHub Team
#[derive(Debug, Default, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
struct Team {
    id: TeamId,
    name: String,
    /// Child teams
    #[serde(default)]
    teams: Vec<TeamId>,
    #[serde(default)]
    members: Vec<MemberId>,
    #[serde(default)]
    /// Parent Teams
    parent: Option<ParentTeam>,
}

impl TryFrom<Value> for Team {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> std::prelude::v1::Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Serialize)]
struct ParentTeam {
    id: TeamId,
    name: String,
}

impl Team {
    fn members(&self) -> Vec<MemberId> {
        self.members.clone()
    }
}

/// Team representation sent to Splunk
#[derive(Debug, Deserialize, Serialize)]
struct TeamSerialize {
    organisation: String,
    name: String,
    id: u64,
    /// Members including child team members
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    members: Vec<MemberSerialize>,
    /// Child teams
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    teams: Vec<ChildTeamSerialize>,
    /// Parent team
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<ParentTeam>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChildTeamSerialize {
    name: String,
    id: u64,
}

impl ToHecEvents for TeamSerialize {
    type Item = TeamSerialize;

    fn source(&self) -> &str {
        "SSPHP:github:teams"
    }

    fn sourcetype(&self) -> &str {
        "github"
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(std::iter::once(self))
    }

    fn ssphp_run_key(&self) -> &str {
        "github"
    }
}

/// A GitHub UserID
#[derive(Hash, Debug, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct MemberId(u64);

/// A GitHub User
#[derive(Debug, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct Member {
    id: MemberId,
    /// GitHub Username
    login: String,
}

impl TryFrom<Value> for Member {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> std::prelude::v1::Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

/// Team representation sent to Splunk
/// Sigle field containing the members `login` / username
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
struct MemberSerialize {
    login: String,
}

impl From<&Member> for MemberSerialize {
    fn from(value: &Member) -> Self {
        Self {
            login: value.login.to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use data_ingester_splunk::splunk::HecEvent;

    use crate::teams::{
        GitHubTeamsOrg, Member, MemberId, MemberSerialize, ParentTeam, Team, TeamId, TeamSerialize,
    };

    fn teams_hec_events() -> Vec<HecEvent> {
        let mut org = GitHubTeamsOrg {
            teams: HashMap::from([
                (
                    TeamId(1),
                    Team {
                        id: TeamId(1),
                        name: "team1".to_string(),
                        members: vec![MemberId(1)],
                        ..Default::default()
                    },
                ),
                (
                    TeamId(2),
                    Team {
                        id: TeamId(2),
                        name: "team2".to_string(),
                        members: vec![MemberId(2)],
                        ..Default::default()
                    },
                ),
                (
                    TeamId(3),
                    Team {
                        id: TeamId(3),
                        name: "team3".to_string(),
                        teams: vec![TeamId(4)],
                        members: vec![MemberId(3)],
                        ..Default::default()
                    },
                ),
                (
                    TeamId(4),
                    Team {
                        id: TeamId(4),
                        name: "team4".to_string(),
                        members: vec![MemberId(4)],
                        parent: Some(ParentTeam {
                            id: TeamId(3),
                            name: "team3".to_string(),
                        }),
                        ..Default::default()
                    },
                ),
            ]),
            members: HashMap::from([
                (
                    MemberId(1),
                    Member {
                        id: MemberId(1),
                        login: "member1".to_string(),
                    },
                ),
                (
                    MemberId(2),
                    Member {
                        id: MemberId(2),
                        login: "member2".to_string(),
                    },
                ),
                (
                    MemberId(3),
                    Member {
                        id: MemberId(3),
                        login: "member3".to_string(),
                    },
                ),
                (
                    MemberId(4),
                    Member {
                        id: MemberId(4),
                        login: "member4".to_string(),
                    },
                ),
            ]),
            name: "Test Org".into(),
        };

        let hec_events = org.team_members_hec_events().unwrap();
        assert_eq!(hec_events.len(), 4);
        hec_events
    }

    #[test]
    fn test_teams_source_sourcetype() {
        let hec_events = teams_hec_events();
        assert!(hec_events
            .iter()
            .all(|event| event.source == "SSPHP:github:teams"));
        assert!(hec_events.iter().all(|event| event.sourcetype == "github"));
    }

    fn teams_serialized() -> Vec<TeamSerialize> {
        teams_hec_events()
            .iter()
            .map(|event| serde_json::from_str(event.event.as_str()).unwrap())
            .collect()
    }

    #[test]
    fn test_teams_parents() {
        for team in teams_serialized() {
            match &team.id {
                1 => {
                    assert!(team.parent.is_none());
                }
                2 => {
                    assert!(team.parent.is_none());
                }
                3 => {
                    assert!(team.parent.is_none());
                }
                4 => {
                    assert!(team.parent.is_some());
                    assert!(team.parent.unwrap().id == TeamId(3));
                }
                _ => {
                    unreachable!()
                }
            }
        }
    }

    #[test]
    fn test_teams_members() {
        for team in teams_serialized() {
            match &team.id {
                1 => {
                    assert!(!team.members.is_empty());
                    assert!(team.members.contains(&MemberSerialize {
                        login: "member1".into()
                    }));
                }
                2 => {
                    assert!(!team.members.is_empty());
                    assert!(team.members.contains(&MemberSerialize {
                        login: "member2".into()
                    }));
                }
                3 => {
                    assert!(!team.members.is_empty());
                    assert!(team.members.contains(&MemberSerialize {
                        login: "member3".into()
                    }));
                    assert!(team.members.contains(&MemberSerialize {
                        login: "member4".into()
                    }));
                }
                4 => {
                    assert!(!team.members.is_empty());
                    assert!(team.members.contains(&MemberSerialize {
                        login: "member4".into()
                    }));
                }
                _ => {}
            }
        }
    }
}
