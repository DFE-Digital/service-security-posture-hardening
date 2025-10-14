// #[cfg(test)]
// mod test {
//     use serde::Deserialize;
//     use serde::Serialize;

//     //    use super::User;

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     pub struct Users {
//         users: Vec<User>,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct User {
//         #[serde(rename = "_links")]
//         pub links: Links,
//         pub descriptor: String,
//         pub directory_alias: Option<String>,
//         pub display_name: String,
//         pub domain: String,
//         pub mail_address: String,
//         pub origin: String,
//         pub origin_id: String,
//         pub principal_name: String,
//         pub subject_kind: String,
//         pub url: String,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct Links {
//         pub avatar: Avatar,
//         pub membership_state: MembershipState,
//         pub memberships: Memberships,
//         #[serde(rename = "self")]
//         pub self_field: SelfField,
//         pub storage_key: StorageKey,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct Avatar {
//         pub href: String,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct MembershipState {
//         pub href: String,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct Memberships {
//         pub href: String,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct SelfField {
//         pub href: String,
//     }

//     #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     pub struct StorageKey {
//         pub href: String,
//     }

//     static USER: &str = r#"{
//   "_links": {
//     "avatar": {
//       "href": "https://dev.azure.com/aktest0831/_apis/GraphProfile/MemberAvatars/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
//     },
//     "membershipState": {
//       "href": "https://vssps.dev.azure.com/aktest0831/_apis/Graph/MembershipStates/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
//     },
//     "memberships": {
//       "href": "https://vssps.dev.azure.com/aktest0831/_apis/Graph/Memberships/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
//     },
//     "self": {
//       "href": "https://vssps.dev.azure.com/aktest0831/_apis/Graph/Users/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
//     },
//     "storageKey": {
//       "href": "https://vssps.dev.azure.com/aktest0831/_apis/Graph/StorageKeys/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
//     }
//   },
//   "descriptor": "aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl",
//   "directoryAlias": "sam.pritchard_education.gov.uk#EXT#",
//   "displayName": "sam.pritchard@education.gov.uk",
//   "domain": "1ed6d920-41e7-46ff-83c2-cd1e713b1c4c",
//   "mailAddress": "sam.pritchard@education.gov.uk",
//   "origin": "aad",
//   "originId": "a0475fb1-0253-4683-aff8-c7a40aa03745",
//   "principalName": "sam.pritchard@education.gov.uk",
//   "subjectKind": "user",
//   "url": "https://vssps.dev.azure.com/aktest0831/_apis/Graph/Users/aad.NmM1YmEyNzktNmJhYi03MWJlLWIwMzItZTA5ZGU1OWE5M2Rl"
// }"#;

//     #[test]
//     fn test_acl_from_json() {
//         let user: User = serde_json::from_str(USER).unwrap();
//         assert_eq!(user.display_name, "sam.pritchard@education.gov.uk");
//     }
// }
