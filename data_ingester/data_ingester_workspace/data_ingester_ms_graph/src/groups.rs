// use crate::splunk::ToHecEvents;
use serde::Deserialize;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::borrow::Cow;

use data_ingester_splunk::splunk::ToHecEvents;

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
    pub(crate) visibility: Option<String>,
}

impl<'a> ToHecEvents for &Groups<'a> {
    type Item = Cow<'a, Group>;
    fn source(&self) -> &str {
        "msgraph"
    }

    fn sourcetype(&self) -> &str {
        "SSPHP.AAD.group"
    }

    // fn to_hec_events(&self) -> anyhow::Result<Vec<crate::splunk::HecEvent>> {
    //     Ok(self
    //         .inner
    //         .iter()
    //         .map(|i| HecEvent::new(&i, self.source(), self.sourcetype()).unwrap())
    //         .collect())
    // }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.inner.iter())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Groups<'a> {
    #[serde(rename = "value")]
    pub inner: Vec<Cow<'a, Group>>,
}

impl Groups<'_> {
    pub fn ids(&self) -> Vec<&'_ String> {
        self.inner.iter().map(|group| &group.id).collect()
    }
}

#[test]
fn group_role_ids() {
    let group1 = Group {
        id: "id_1".to_owned(),
        display_name: None,
        visibility: None,
    };
    let group2 = Group {
        id: "id_2".to_owned(),
        display_name: None,
        visibility: None,
    };
    let groups = [group1, group2];
    let groups = groups.iter().collect::<Groups>();
    assert_eq!(groups.ids(), ["id_1", "id_2"]);
}

impl<'a> FromIterator<&'a Group> for Groups<'a> {
    fn from_iter<I: IntoIterator<Item = &'a Group>>(iter: I) -> Self {
        let mut inner = vec![];
        for i in iter {
            inner.push(Cow::Borrowed(i));
        }
        Self { inner }
    }
}
