# Graph data we know how to collect but have chosen not to

Candidates, not a backlog. Nothing here is collected because nothing has needed it yet. If a
detection or a use case turns out to need one, add it then — each is a `ms_graph.toml` entry, not
Rust.

Kept beside the collector so the answer to "can we get X?" is next to the code that would get it.

---

## Properties left out of the application feeds

`servicePrincipals` and `applications` are collected with a `$select` rather than whole. The
`Application.Read.All` permission is already granted, so these are available whenever wanted:

| Property | On | Answers |
|---|---|---|
| `keyCredentials`, `passwordCredentials` | applications | certificate and secret expiry across ~7,750 registrations |
| `requiredResourceAccess` | applications | what permissions each registration asks for |
| `appRoles`, `oauth2PermissionScopes` | servicePrincipals | what each app can grant |
| `replyUrls` | both | redirect surface |
| `owners` | both | who owns an app, and which have none |

**Do not append these to the existing `$select`.** The reason the feeds are selective is not
tidiness. Splunk stops extracting JSON fields after `[kv] maxchars`, 10,240 characters by default,
and a record past that point arrives whole, matches keyword searches, and yields *no fields at
all* — it looks healthy by event count while being invisible to every field-based search. That
cost 23.8% of user records until it was found in October 2026. `appRoles` and
`oauth2PermissionScopes` on a large application run to tens of thousands of characters, across a
feed of ~14,000 service principals.

Each wants **its own toml entry**, selecting `id`, `appId` and that one property, so the record
stays inside the extraction window. Join back on `appId` — that is the client id a Conditional
Access policy references; `id` is the directory object id and will not match.

Note for `keyCredentials` on service principals specifically: Graph does not return the `key`
value when listing, and selecting it carries a documented throttling limit of 150 requests per
minute per tenant.

---

## `/policies/` children we do not collect

We collect nine of the `policyRoot` children. These six we do not. Each is one `ms_graph.toml`
entry, and `Policy.Read.All` — already granted — covers them.

| Endpoint | Answers |
|---|---|
| `/v1.0/policies/crossTenantAccessPolicy` | per-partner B2B settings: inbound and outbound access, and whether MFA satisfied in the partner tenant is trusted here |
| `/v1.0/policies/appManagementPolicies` | restrictions on app and service principal credentials, such as banning password secrets or capping certificate lifetime |
| `/v1.0/policies/homeRealmDiscoveryPolicies` | which applications bypass the Entra sign-in page and go straight to a federated identity provider |
| `/v1.0/policies/tokenIssuancePolicies` | SAML token signing behaviour for individual applications |
| `/v1.0/policies/claimsMappingPolicies` | custom claims issued in tokens for individual applications |
| `/v1.0/policies/featureRolloutPolicies` | who is opted into a staged rollout, for example passwordless or a change of cloud authentication method |

Descriptions are of the Graph resources, not of anything measured here — nothing in this list is
collected, so none of it has been seen in this tenant.
