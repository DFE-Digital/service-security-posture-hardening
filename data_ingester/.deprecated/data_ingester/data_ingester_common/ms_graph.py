import asyncio
import json
from azure.identity.aio import ClientSecretCredential
from kiota_authentication_azure.azure_identity_authentication_provider import (
    AzureIdentityAuthenticationProvider,
)
from msgraph import GraphRequestAdapter
from msgraph import GraphServiceClient
from .splunk import Splunk, HecEvent
import kiota_serialization_json
import copy
import sys
import logging

logger = logging.getLogger("data_ingesterms_graph")
logging.basicConfig()
logger.setLevel(logging.INFO)

class AzureGroup:
    def __init__(self, role):
        self._role = role

    def to_dict(self):
        d = {
            "id": self._role["id"],
            # "createdByAppId": "74658136-14ec-4630-ad9b-26e160ff0fc6",
            # "createdDateTime": "2023-07-21T13:40:28+00:00",
            "displayName": self._role["displayName"],
            # "groupTypes": [],
            # "infoCatalogs": [],
            # "mailEnabled": false,
            # "mailNickname": "bf2085d1-f",
            # "onPremisesProvisioningErrors": [],
            # "organizationId": "1ed6d920-41e7-46ff-83c2-cd1e713b1c4c",
            # "proxyAddresses": [],
            # "renewedDateTime": "2023-07-21T13:40:28+00:00",
            # "resourceBehaviorOptions": [],
            # "resourceProvisioningOptions": [],
            "securityEnabled": self._role["securityEnabled"],
            # "securityIdentifier": "S-1-12-1-2248411536-1272927851-1856556721-1143733892",
            # "serviceProvisioningErrors": [],
            # "writebackConfiguration": {},
        }
        return d


class AzureRole:
    def __init__(self, role):
        self._role = role

    def is_privileged(self):
        return self._role["roleDefinition"]["isPrivileged"]

    def to_dict(self):
        d = {
            "id": self._role["id"],
            # "@odata.type": "#microsoft.graph.directoryRole",
            "description": self._role["description"],
            "displayName": self._role["displayName"],
            # "roleTemplateId": "9f06204d-73c1-4d4c-880a-6edb90606fd8",
            "roleDefinition": {
                "id": self._role["roleDefinition"]["id"],
                "isPrivileged": self._role["roleDefinition"]["isPrivileged"],
                # "inheritsPermissionsFrom@odata.context": "https://graph.microsoft.com/beta/$metadata#roleManagement/directory/roleDefinitions('9f06204d-73c1-4d4c-880a-6edb90606fd8')/inheritsPermissionsFrom",
                # "description": "Users assigned to this role are added to the local administrators group on Azure AD-joined devices.",
                # "displayName": "Azure AD Joined Device Local Administrator",
                "inheritsPermissionsFrom": self._role["roleDefinition"][
                    "inheritsPermissionsFrom"
                ],
                "isBuiltIn": self._role["roleDefinition"]["isBuiltIn"],
                "isEnabled": self._role["roleDefinition"]["isEnabled"],
                "resourceScopes": self._role["roleDefinition"]["resourceScopes"],
                # "rolePermissions": [
                #     {
                #         "allowedResourceActions": [
                #             "microsoft.directory/groupSettings/standard/read",
                #             "microsoft.directory/groupSettingTemplates/standard/read",
                #         ]
                #     }
                # ],
                # "templateId": "9f06204d-73c1-4d4c-880a-6edb90606fd8",
                # "version": "1",
            },
        }
        return d


class AzureUser:
    def __init__(self, user):
        # self._user = user
        self._roles = []
        self._groups = []
        self._cap = []
        self._user = user

    def id(self):
        return self._user["id"]

    def set_groups(self, groups):
        self._groups = groups
        return self

    def add_group(self, group):
        self._groups.append(AzureGroup(group))

    def groups(self):
        return self._groups

    def set_roles(self, roles):
        self._roles = [AzureRole(role) for role in roles]
        return self

    def add_role(self, role):
        self._roles.append(AzureRole(role))

    def add_conditional_access_policy(self, cap):
        self._cap.append(cap)

    def is_privileged(self):
        return any([role.is_privileged() for role in self._roles])

    def to_dict(self):
        d = {
            "conditionalAccessPolicies": self._cap,
            "groups": [group.to_dict() for group in self._groups],
            "isPrivileged": self.is_privileged(),
            "roles": [role.to_dict() for role in self._roles],
            "id": self._user["id"],
            # "@odata.type": "#microsoft.graph.user",
            # "isLicenseReconciliationNeeded": false,
            # "onPremisesObjectIdentifier": null,
            # "externalUserConvertedOn": null,
            # "cloudRealtimeCommunicationInfo": {"isSipEnabled": null},
            # "onPremisesSipInfo": {
            #     "isSipEnabled": false,
            #     "sipDeploymentLocation": null,
            #     "sipPrimaryAddress": null,
            # },
            "accountEnabled": self._user["accountEnabled"],
            # "assignedLicenses": [
            #     {"disabledPlans": [], "skuId": "84a661c4-e949-4bd2-a560-ed7766fcaf2b"}
            # ],
            "assignedPlans": self._user["assignedPlans"],
            "authorizationInfo": self._user["authorizationInfo"],
            # "businessPhones": [],
            "createdDateTime": self._user["createdDateTime"],
            # "deviceKeys": [],
            "displayName": self._user["displayName"],
            # "identities": [
            #     {
            #         "issuer": "aksecondad.onmicrosoft.com",
            #         "issuerAssignedId": "useringroup3@aksecondad.onmicrosoft.com",
            #         "signInType": "userPrincipalName",
            #     }
            # ],
            # "imAddresses": [],
            # "infoCatalogs": [],
            # "mailNickname": "useringroup3",
            # "onPremisesExtensionAttributes": {},
            # "onPremisesProvisioningErrors": [],
            # "otherMails": [],
            # "passwordProfile": self._user["passwordProfile"],
            # {
            #     "forceChangePasswordNextSignIn": true,
            #     "forceChangePasswordNextSignInWithMfa": false,
            # },
            # "provisionedPlans": [],
            # "proxyAddresses": [],
            # "refreshTokensValidFromDateTime": "2023-07-24T15:25:20+00:00",
            # "securityIdentifier": "S-1-12-1-642842178-1276360934-1954373002-2693348416",
            # "serviceProvisioningErrors": [],
            # "signInSessionsValidFromDateTime": "2023-07-24T15:25:20+00:00",
            # "usageLocation": "GB",
            "userPrincipalName": self._user["userPrincipalName"],
            "userType": self._user["userType"],
        }

        missing_attributes = [
            "givenName",
            "surname",
            "passwordProfile",
        ]

        for attribute in missing_attributes:
            if attribute in self._user:
                d[attribute] = self._user[attribute]
        return d


class Azure:
    def __init__(self, splunk, source=None, sourcetype=None, host=None, tenant_id=None, client_id=None, client_secret=None):
        self.splunk = splunk
        # self._user = user
        self._roles = []
        self._groups = []
        self._cap = []
        self.source = source
        self.sourcetype = sourcetype
        self.host = host
        self.tenant_id = tenant_id
        self.client_id = client_id
        self.client_secret = client_secret
        
        self.client()

    def client(self):
        credential = ClientSecretCredential(
            self.tenant_id,
            self.client_id,
            self.client_secret,
        )

        scopes = ["https://graph.microsoft.com/.default"]
        auth_provider = AzureIdentityAuthenticationProvider(credential, scopes=scopes)
        request_adapter = GraphRequestAdapter(auth_provider)
        self.client = GraphServiceClient(request_adapter)

    def add_to_splunk(self, collection, source=None, sourcetype=None, host=None):
        if not source:
            source = self.source
        if not sourcetype:
            sourcetype = self.sourcetype
        if not host:
            host = self.host

        if not isinstance(collection, list):
            collection = [collection]

        hec_events = HecEvent.events(
            collection, source=source, sourcetype=sourcetype, host=host
        )
        self.splunk.extend(hec_events)

    async def get_users(self, directory_roles):
        collection = await self.client.users.get()
        users = []
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            value.serialize(kjsonwriter)
            content = json.loads(kjsonwriter.get_serialized_content())
            self.add_to_splunk(content, sourcetype="users.get")
            user = AzureUser(content)
            result = await self.client.users.by_user_id(
                user.id()
            ).transitive_member_of.get()
            for value in result.value:
                kjsonfactory = (
                    kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
                )
                kjsonwriter = kjsonfactory.get_serialization_writer(
                    kjsonfactory.get_valid_content_type()
                )
                value.serialize(kjsonwriter)
                content = json.loads(kjsonwriter.get_serialized_content())
                if content["@odata.type"] == "#microsoft.graph.group":
                    user.add_group(content)
                else:
                    user.add_role(directory_roles[content["id"]])
            users.append(user)
        return users

    async def get_directory_roles(self):
        collection = await self.client.directory_roles.get()
        directory_roles = {}
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            value.serialize(kjsonwriter)
            content = json.loads(kjsonwriter.get_serialized_content())
            directory_roles[content["id"]] = copy.deepcopy(content)
            self.add_to_splunk(content, sourcetype="directory_roles.get")

        return directory_roles

    async def get_role_management_assignments(self):
        collection = await self.client.role_management.directory.role_assignments.get()
        assignments = []
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            value.serialize(kjsonwriter)
            content = json.loads(kjsonwriter.get_serialized_content())
            self.add_to_splunk(content, sourcetype="role_management_assignments.get")
            assignments.append(content)
        return assignments

    async def get_role_management_definitions(self):
        collection = await self.client.role_management.directory.role_definitions.get()
        role_definitions = {}
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            value.serialize(kjsonwriter)
            content = json.loads(kjsonwriter.get_serialized_content())
            role_definitions[content["id"]] = copy.deepcopy(content)
            # self.add_to_splunk(content, sourcetype="role_management.definitions.get")
        return role_definitions

    async def get_domains(self):
        collection = await self.client.domains.get()
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            value.serialize(kjsonwriter)
            content = json.loads(kjsonwriter.get_serialized_content())
            self.add_to_splunk(content, sourcetype="domains.get")

    async def get_policies_conditional_access(self):
        cap = []
        collection = await self.client.identity.conditional_access.policies.get()
        for value in collection.value:
            kjsonfactory = (
                kiota_serialization_json.json_serialization_writer_factory.JsonSerializationWriterFactory()
            )
            kjsonwriter = kjsonfactory.get_serialization_writer(
                kjsonfactory.get_valid_content_type()
            )
            # Serializer chokes on UUID, so we convert to str
            value.additional_data['templateId'] = str(value.additional_data['templateId'])
            value.serialize(kjsonwriter)
            tmp = kjsonwriter.get_serialized_content()
            content = json.loads(tmp)
            self.add_to_splunk(content, sourcetype="policies.conditional_access.get")
            cap.append(content)
        return cap

    async def get_all(self):
        logging.info("9")
        role_definitions = await self.get_role_management_definitions()
        directory_roles = await self.get_directory_roles()
        cap = await self.get_policies_conditional_access()
        for k, v in directory_roles.items():
            v["roleDefinition"] = role_definitions[v["roleTemplateId"]]
            logging.info("10")
        users = await self.get_users(directory_roles)

        # await self.get_domains()
        role_assignments = await self.get_role_management_assignments()

        for c in cap:
            cn = ConditionalAccessPolicy(c)
            cn.users([user.id() for user in users])
            for user in users:
                if user.id() in cn.affected_users():
                    user.add_conditional_access_policy(cn.to_dict())

            # users_for_json = [
            #     {
            #         "id": u['id'],
            #         "displayName": u['displayName'],
            #         "isPrivileged": any([ role['roleDefinition']['isPrivileged'] for role in (u['SSPHP_transitive_directory_roles'] if 'SSPHP_transitive_directory_roles' in u else [])])
            #     }
            #     for u in users
            # ]
            # data = {"capId": c['id'], "affected_users": users_for_json, "displayName": c['displayName']}
            # self.add_to_splunk(cn.to_dict(), sourcetype="SSPHP.conditional_access_policy.affected_users")
        logging.info("13")
        self.add_to_splunk(
            [user.to_dict() for user in users], sourcetype="SSPHP.AAD.user"
        )
        logging.info("14")

    async def run(self):
        logging.info("15")
        await self.get_all()
        logging.info("16")        


class ConditionalAccessPolicy:
    def __init__(self, cap):
        self.cap = cap
        self.user_ids = []
        self._affected_users = set()

    def users(self, user_ids):
        self.user_ids.extend(user_ids)

    def user(self, user_id):
        self.user_ids.append(user_id)

    def to_dict(self):
        cap = {
            self.cap['id']: self.cap['state'][0],
            "id": self.cap["id"],
            # "templateId": null,
            # "conditions": {
            #     "applications": {
            #         "excludeApplications": [],
            #         "includeApplications": ["None"],
            #         "includeAuthenticationContextClassReferences": [],
            #         "includeUserActions": [],
            #     },
            #     "locations": {
            #         "excludeLocations": ["04c34872-5d2f-4e4f-92d7-ae115195263f"],
            #         "includeLocations": ["All"],
            #     },
            #     "platforms": {},
            #     "users": {
            #         "excludeGroups": [],
            #         "excludeRoles": [],
            #         "excludeUsers": [],
            #         "includeGroups": [],
            #         "includeRoles": [],
            #         "includeUsers": ["2650fe42-b8e6-4c13-8a5d-7d74403c89a0"],
            #     },
            #     "times": null,
            # },
            # "createdDateTime": "2023-07-27T15:47:08.260417+00:00",
            "displayName": self.cap["displayName"],            
            # "grantControls": {
            #     "authenticationStrength": {
            #         "id": "00000000-0000-0000-0000-000000000003",
            #         "combinationConfigurations@odata.context": "https://graph.microsoft.com/beta/$metadata#identity/conditionalAccess/policies('1eba8b95-3fc9-434a-a645-97ebc3cbbe54')/grantControls/authenticationStrength/combinationConfigurations",
            #         "combinationConfigurations": [],
            #         "createdDateTime": "2021-12-01T08:00:00+00:00",
            #         "description": "Passwordless methods that satisfy strong authentication, such as Passwordless sign-in with the Microsoft Authenticator",
            #         "displayName": "Passwordless MFA",
            #         "modifiedDateTime": "2021-12-01T08:00:00+00:00",
            #         "policyType": ["builtIn"],
            #         "requirementsSatisfied": ["mfa"],
            #     },
            #     "customAuthenticationFactors": [],
            #     "operator": "OR",
            #     "termsOfUse": [],
            #     "authenticationStrength@odata.context": "https://graph.microsoft.com/beta/$metadata#identity/conditionalAccess/policies('1eba8b95-3fc9-434a-a645-97ebc3cbbe54')/grantControls/authenticationStrength/$entity",
            # },
            # "modifiedDateTime": "2023-08-01T10:59:57.198613+00:00",
            "state": self.cap["state"],
        }
        return cap

    def affected_users(self):
        # By User
        users = self.user_ids
        for user in users:
            if "All" in self.cap["conditions"]["users"]["includeUsers"]:
                self._affected_users.add(user)
                continue
            if user in self.cap["conditions"]["users"]["includeUsers"]:
                self._affected_users.add(user)
                continue

        # By Group
        for user in users:
            if "SSPHP_transitive_member_of" not in user:
                continue
            for group in user["SSPHP_transitive_member_of"]:
                if group["id"] in self.cap["conditions"]["users"]["includeGroups"]:
                    self._affected_users.add(user)
        # By role
        for user in users:
            if "SSPHP_transitive_directory_role" not in user:
                continue
            for role in user["SSPHP_transitive_directory_role"]:
                if role["id"] in self.cap["conditions"]["users"]["includeRoles"]:
                    self._affected_users.add(user)

        ## Remove
        for user in users:
            if "All" in self.cap["conditions"]["users"]["excludeUsers"]:
                try:
                    self._affected_users.remove(user)
                except KeyError:
                    pass
                continue
            if user in self.cap["conditions"]["users"]["excludeUsers"]:
                try:
                    self._affected_users.remove(user)
                except KeyError:
                    pass
                continue

        for user in users:
            if "SSPHP_transitive_member_of" not in user:
                continue
            for group in user["SSPHP_transitive_member_of"]:
                if group["id"] in self.cap["conditions"]["users"]["excludeGroups"]:
                    try:
                        self._affected_users.remove(user)
                    except KeyError:
                        pass

        for user in users:
            if "SSPHP_transitive_directory_role" not in user:
                continue
            for role in user["SSPHP_transitive_directory_role"]:
                if role["id"] in self.cap["conditions"]["users"]["excludeRoles"]:
                    try:
                        self._affected_users.remove(user)
                    except KeyError:
                        pass

        return self._affected_users
