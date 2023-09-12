// use crate::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::borrow::Cow;

// https://learn.microsoft.com/en-us/graph/api/resources/group?view=graph-rest-1.0
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub(crate) id: String,
    // "classification": Null,
    // "createdDateTime": String("2023-07-21T13:40:56Z"),
    // "creationOptions": Array [],
    // "deletedDateTime": Null,
    // "description": Null,
    pub(crate) display_name: Option<String>,
    // "expirationDateTime": Null,
    // "groupTypes": Array [],
    // "isAssignableToRole": Null,
    // "mail": Null,
    // "mailEnabled": Bool(false),
    // "mailNickname": String("ad7b363e-c"),
    // "membershipRule": Null,
    // "membershipRuleProcessingState": Null,
    // "onPremisesDomainName": Null,
    // "onPremisesLastSyncDateTime": Null,
    // "onPremisesNetBiosName": Null,
    // "onPremisesProvisioningErrors": Array [],
    // "onPremisesSamAccountName": Null,
    // "onPremisesSecurityIdentifier": Null,
    // "onPremisesSyncEnabled": Null,
    // "preferredDataLocation": Null,
    // "preferredLanguage": Null,
    // "proxyAddresses": Array [],
    // "renewedDateTime": String("2023-07-21T13:40:56Z"),
    // "resourceBehaviorOptions": Array [],
    // "resourceProvisioningOptions": Array [],
    // "securityEnabled": Bool(true),
    // "securityIdentifier": String("S-1-12-1-4284624009-1147235088-2041966773-2552019788"),
    // "serviceProvisioningErrors": Array [],
    // "theme": Null,
    // "visibility": Null,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Groups<'a> {
    pub value: Vec<Cow<'a, Group>>,
}

impl Groups<'_> {
    pub fn new() -> Self {
        Self { value: Vec::new() }
    }

    pub fn ids(&self) -> Vec<&'_ String> {
        self.value.iter().map(|group| &group.id).collect()
    }
}

// impl ToHecEvents for Groups<'_> {
//     fn source() -> &'static str {
//         "msgraph"
//     }

//     fn sourcetype() -> &'static str {
//         "msgraph:group"
//     }
// }

// use std::ops::Deref;
// impl<'a> Deref for Groups<'a> {
//   type Target = [Group];

//   fn deref(&self) -> &[Cow<'_, Group] {
//     &self.value[..]
//   }
// }

// impl<'a> IntoIterator for Groups<'a> {
//     type Item = Group;
//     type IntoIter = std::vec::IntoIter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.value.into_iter()
//     }
// }

// impl<'a> Iterator for Group<'a> {
//     type Item = Group;
//     fn next(&mut self)
// }

#[test]
fn group_role_ids() {
    let group1 = Group {
        id: "id_1".to_owned(),
        display_name: None,
    };
    let group2 = Group {
        id: "id_2".to_owned(),
        display_name: None,
    };
    let groups = vec![group1, group2];
    let groups = groups.iter().collect::<Groups>();
    assert_eq!(groups.ids(), ["id_1", "id_2"]);
}

impl<'a> FromIterator<&'a Group> for Groups<'a> {
    fn from_iter<I: IntoIterator<Item = &'a Group>>(iter: I) -> Self {
        let mut value = vec![];
        for i in iter {
            value.push(Cow::Borrowed(i));
        }
        Self { value }
    }
}
