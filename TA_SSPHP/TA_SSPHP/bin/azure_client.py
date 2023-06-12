from azure.identity import ClientSecretCredential
from azure.mgmt.resource import ResourceManagementClient
from azure.mgmt.security import SecurityCenter
from azure.mgmt.resource.subscriptions import SubscriptionClient
from azure.mgmt.resourcegraph import ResourceGraphClient
from azure.mgmt.resourcegraph.models import (
    QueryRequest,
    QueryRequestOptions,
    ResultFormat,
)
from azure.core.exceptions import ServiceRequestError
import time


class AzureClient:
    def __init__(self, *args, **kwargs):
        self._credentials = None
        self._security_center = {}
        super().__init__(*args, **kwargs)

    def get_azure_credentials(self):
        """Create an Azure session"""
        if self._credentials:
            return self._credentials

        global_account = self.get_arg("azure_app_account")
        tenant_id = self.get_arg("tenant_id")

        self._credentials = ClientSecretCredential(
            tenant_id,
            global_account["username"],  # client ID
            client_secret=global_account["password"],
            # No provision for .gov azure
        )

        return self._credentials

    def security_center(self, subscription_id, caller):
        sc = self._security_center.setdefault(subscription_id, {}).get(caller, None)

        if not sc:
            sc = SecurityCenter(self.get_azure_credentials(), subscription_id)
            self._security_center[subscription_id].update({caller: sc})

        return sc

    def get_subscriptions(self):
        subscriptions = SubscriptionClient(
            self.get_azure_credentials()
        ).subscriptions.list()
        return subscriptions

    def subscription_ids(self, subscripitons):
        return [subscripiton.subscription_id for subscripiton in subscripitons]

    def get_resource_groups(self, subscription_id):
        resource_groups = ResourceManagementClient(
            self.get_azure_credentials(), subscription_id
        ).resource_groups.list()
        return list(resource_groups)

    def get_assessments(self, subscription_id):
        """Get security center assessments"""
        assessments = self.security_center(
            subscription_id, "assessments"
        ).assessments.list(f"/subscriptions/{subscription_id}")
        return assessments

    def get_all_sub_assessments(self, subscription_id):
        """Get security center assessments"""
        assessments = self.security_center(
            subscription_id, "sub_assessments"
        ).sub_assessments.list_all(f"/subscriptions/{subscription_id}")
        return assessments

    def get_assessment_metadata(self, subscription_id):
        assessment_metadata = self.security_center(
            subscription_id, "assessment_metadata"
        ).assessments_metadata.list()
        return assessment_metadata

    def list_alerts(self, subscription_id):
        alerts = self.security_center(subscription_id, "alerts").alerts.list()
        return alerts

    def list_secure_scores(self, subscription_id):
        scores = self.security_center(
            subscription_id, "securescore"
        ).secure_scores.list()
        return scores

    def list_secure_score_controls(self, subscription_id):
        sscs = self.security_center(
            subscription_id, "securescorecontrols"
        ).secure_score_controls.list()
        return sscs

    def list_secure_score_control_definitions(self, subscription_id):
        ssd = self.security_center(
            subscription_id, "securescorecontroldefinitions"
        ).secure_score_control_definitions.list()
        return ssd

    def list_settings(self, subscription_id):
        scores = self.security_center(
            subscription_id, "settings"
        ).secure_score_controls.list()
        return scores

    def get_resource_graph(self, subscription_id, event_writer):
        error_count = 0
        tables = [
            "Resources",
            "ResourceContainers",
            "AdvisorResources",
            "AlertsManagementResources",
            "DesktopVirtualizationResources",
            "ExtendedLocationResources",
            "GuestConfigurationResources",
            "HealthResources",
            "IoTSecurityResources",
            "KubernetesConfigurationResources",
            "MaintenanceResources",
            "PatchAssessmentResources",
            "PatchInstallationResources",
            "PolicyResources",
            "RecoveryServicesResources",
            "SecurityResources",
            "ServiceHealthResources",
            "WorkloadMonitorResources",
        ]
        data = {}
        for table in tables:
            while True:
                try:
                    client = ResourceGraphClient(self.get_azure_credentials())
                except ServiceRequestError as e:
                    event_writer.log(
                        "ERROR", "Error while building resource client graph: " + str(e)
                    )
                    error_count += 1
                    if error_count < 5:
                        time.sleep(5)
                        continue
                    raise

                query = f"{table} | order by name asc"
                request = QueryRequest(query=query, subscriptions=[subscription_id])
                try:
                    resource_graphs = client.resources(request)
                except ServiceRequestError as e:
                    event_writer.log(
                        "ERROR", "Error while requesting resource graph: " + str(e)
                    )
                    error_count += 1
                    if error_count < 5:
                        time.sleep(5)
                        continue
                    raise

                table_data = resource_graphs.data

                while resource_graphs.skip_token:
                    options = QueryRequestOptions(skip_token=resource_graphs.skip_token)
                    request = QueryRequest(
                        query=query, subscriptions=[subscription_id], options=options
                    )
                    try:
                        resource_graphs = client.resources(request)
                    except ServiceRequestError as e:
                        event_writer.log(
                            "ERROR", "Error while requesting resource graph: " + str(e)
                        )
                        error_count += 1
                        if error_count < 5:
                            time.sleep(5)
                            continue
                        raise
                    resource_graphs = client.resources(request)
                    table_data += resource_graphs.data
                data.setdefault(table, []).extend(table_data)
                break
        return data
