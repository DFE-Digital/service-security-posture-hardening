# encoding = utf-8
"""

Copyright 2020 Splunk Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

"""
import concurrent.futures
import copy
import json
import os
import re
import sys
import time
from datetime import datetime
import import_declare_test


from azure.core.exceptions import AzureError
from azure.mgmt.security.v2019_01_01_preview.models import SecuritySubAssessment
from azure.mgmt.security.v2021_06_01.models import SecurityAssessmentResponse
from msrestazure.tools import parse_resource_id
from splunklib import modularinput as smi
from splunktaucclib.modinput_wrapper import base_modinput as base_mi
import azure

from azure_client import AzureClient


class SecurityAssessmentResponse(SecurityAssessmentResponse):
    subassessment_resource_scope_regex = re.compile(
        "(?P<scope>.*?)/providers/Microsoft.Security/assessments/(?P<assessment_name>[^/]+)/subAssessments"
    )

    def sub_assessment_link(self):
        if self.additional_data:
            return self.additional_data.get("subAssessmentsLink", None)
        else:
            return None

    def assessment_key(self):
        return self.name

    def sub_assessment_resource_scope(self):
        match = self.subassessment_resource_scope_regex.search(
            self.sub_assessment_link()
        )
        return match.group("scope")

    def subscription_id(self):
        return parse_resource_id(self.id)["subscription"]

    def resource_id(self):
        return self.resource_details.additional_properties.get("Id", "NORESOURCEID")


azure.mgmt.security.v2019_01_01_preview.models.SecuritySubAssessment = (
    SecuritySubAssessment
)

azure.mgmt.security.v2021_06_01.models.SecurityAssessmentResponse = (
    SecurityAssessmentResponse
)


azure.mgmt.security.v2019_01_01_preview.models._models_py3.AdditionalData.enable_additional_properties_sending()
azure.mgmt.security.v2019_01_01_preview.models._models_py3.AzureResourceDetails.enable_additional_properties_sending()
azure.mgmt.security.v2019_01_01_preview.models._models_py3.SubAssessmentStatus.enable_additional_properties_sending()
azure.mgmt.security.v2019_01_01_preview.models.SecuritySubAssessment.enable_additional_properties_sending()
azure.mgmt.security.v2021_06_01.models.ResourceDetails.enable_additional_properties_sending()

bin_dir = os.path.basename(__file__)


class ModInputAzureCloudDefender(AzureClient, base_mi.BaseModInput):
    def __init__(self):
        use_single_instance = False
        super().__init__("ta_ssphp", "azure_cloud_defender", use_single_instance)
        self.executor = concurrent.futures.ThreadPoolExecutor(max_workers=40)
        self.ssphp_run = datetime.now().timestamp()

    def get_scheme(self):
        """overloaded splunklib modularinput method"""
        scheme = super().get_scheme()
        scheme.title = "Azure Cloud Defender"
        scheme.description = "Go to the add-on's configuration UI and configure modular inputs under the Inputs menu."
        scheme.use_external_validation = True
        scheme.streaming_mode_xml = True

        scheme.add_argument(
            smi.Argument("name", title="Name", description="", required_on_create=True)
        )
        scheme.add_argument(
            smi.Argument(
                "azure_app_account",
                title="Azure App Account",
                description="",
                required_on_create=True,
                required_on_edit=False,
            )
        )
        scheme.add_argument(
            smi.Argument(
                "tenant_id",
                title="Tenant ID",
                description="",
                required_on_create=True,
                required_on_edit=False,
            )
        )
        return scheme

    def get_app_name(self):
        return "TA_SSPHP"

    # def validate_input(self, definition):
    #     pass

    # def write_events(self, event_writer, collection, metadata):
    #     """Write a collection of events using the provided eventwriter and metadata"""
    #     for item in collection:
    #         event = self.new_event(
    #             data=json.dumps(item),
    #             source=metadata["source"],
    #             index=metadata["index"],
    #             sourcetype=metadata["sourcetype"],
    #         )
    #         event_writer.write_event(event)
    #     sys.stdout.flush()

    def get_sub_assessments(self, has_sub_assessments):
        if not has_sub_assessments.sub_assessment_link():
            return []
        try:
            sub_assessments = self.security_center(
                has_sub_assessments.subscription_id(), "sub_assessments"
            ).sub_assessments.list(
                has_sub_assessments.sub_assessment_resource_scope(),
                has_sub_assessments.assessment_key(),
            )
        except AzureError as e:
            self.logger.error(e)
            sub_assessments = []
        return sub_assessments

    def smash_has_assessments_sub_assessment(self, has_assessments):
        has_assessments.sub_assessments = []
        try:
            has_assessments.sub_assessments = list(
                self.get_sub_assessments(has_assessments)
            )
        except AzureError as e:
            self.logger.error(e)
        return has_assessments

    def get_subscription_events(self, subscription_id):
        assessments = self.executor.submit(self.get_assessments, subscription_id)

        assessment_metadata = self.executor.submit(
            self.get_assessment_metadata, subscription_id
        )

        secure_score_control_definitions = self.executor.submit(
            self.list_secure_score_control_definitions, subscription_id
        )

        assessments = assessments.result()
        assessments = list(
            self.executor.map(self.smash_has_assessments_sub_assessment, assessments)
        )

        assessment_metadata = list(assessment_metadata.result())

        secure_score_control_definitions = list(
            secure_score_control_definitions.result()
        )

        self.logger.info(
            f"smash_events_subscriptions():{subscription_id} len(assessments)={len(assessments)} len(assessment_metadata)={len(assessment_metadata)}"
        )

        events = []

        for assessment in assessments:
            event = {}

            for metadata in assessment_metadata:
                if metadata.name in assessment.id:
                    event["assessment_metadata"] = metadata.serialize(
                        keep_readonly=True
                    )

                    for sscd in secure_score_control_definitions:
                        for id_ in sscd.assessment_definitions:
                            if metadata.name in id_.id:
                                sscd_serialized = sscd.serialize(keep_readonly=True)
                                del sscd_serialized["properties"][
                                    "assessmentDefinitions"
                                ]
                                event.get("assessment_metadata", {}).setdefault(
                                    "secure_score_control_details", {}
                                ).update(sscd_serialized)

            event["assessment"] = assessment.serialize(keep_readonly=True)

            if hasattr(assessment, "sub_assessments") and assessment.sub_assessments:
                sub_assessments = [
                    sa.serialize(keep_readonly=True)
                    for sa in assessment.sub_assessments
                ]

                for sub_assessment in sub_assessments:
                    sub_event = copy.deepcopy(event)
                    sub_event["sub_assessment"] = sub_assessment

                    events.append(sub_event)
                else:
                    continue

            events.append(event)

        events = self.add_metadata_to_events(events)

        self.logger.info(f"subscription_id:{subscription_id}, events:{len(events)}")

        return events

    def add_metadata_to_events(self, events):
        for event in events:
            assessment_id = event.get("assessment", {}).get("id", None)
            if assessment_id:
                event.setdefault("meta", {}).setdefault("assessment", {}).update(
                    {"id": parse_resource_id(assessment_id)}
                )

            assessment_resource_id = (
                event.get("assessment", {})
                .get("properties", {})
                .get("resourceDetails", {})
                .get("Id", None)
            )
            if assessment_resource_id:
                event.setdefault("meta", {}).setdefault("assessment", {}).update(
                    {"resource_id": parse_resource_id(assessment_resource_id)}
                )

            sub_assessment_id = event.get("sub_assessment", {}).get("id", None)
            if sub_assessment_id:
                event.setdefault("meta", {}).setdefault("sub_assessment", {}).update(
                    {"id": parse_resource_id(sub_assessment_id)}
                )

            sub_assessment_resource_id = (
                event.get("sub_assessment", {})
                .get("properties", {})
                .get("resourceDetails", {})
                .get("id", None)
            )
            if sub_assessment_resource_id:
                event.setdefault("meta", {}).setdefault("sub_assessment", {}).update(
                    {"resource_id": parse_resource_id(sub_assessment_resource_id)}
                )

        return events

    def get_events_threaded(self):
        subscriptions = self.get_subscriptions()

        results = []
        executor = concurrent.futures.ThreadPoolExecutor(max_workers=5)
        for subscription_id in self.subscription_ids(subscriptions):
            results.append(
                executor.submit(self.get_subscription_events, subscription_id)
            )

        metadata = {
            "sourcetype": "azure:security:finding",
            "index": self.get_output_index(),
            "source": f"{self.input_type}",
        }

        count = 0

        events = []

        for r in concurrent.futures.as_completed(results):
            r = r.result()
            for event in r:
                event["SSPHP_RUN"] = self.ssphp_run
                event1 = self.new_event(
                    data=json.dumps(event, sort_keys=True),
                    source=metadata["source"],
                    index=metadata["index"],
                    sourcetype=metadata["sourcetype"],
                )

                self.event_writer.write_event(event1)
                count += 1
                events.append(event)

        sys.stdout.flush()
        self.logger.info(f"Finished writing events: {count}")
        return events

    def collect_events(self, event_writer):
        self.event_writer = event_writer
        t1 = time.perf_counter()
        events = self.get_events_threaded()
        t2 = time.perf_counter()
        self.logger.info(
            f"time: smash_events_threaded:{t2-t1}"  # process_smashed_events:{t3-t2}, t3-t4:{t4-t3}, write_events:{t5-t3}"
        )
        return events

    def get_account_fields(self):
        account_fields = []
        account_fields.append("azure_app_account")
        return account_fields

    def get_checkbox_fields(self):
        checkbox_fields = []
        return checkbox_fields

    # def get_global_checkbox_fields(self):
    #     if self.global_checkbox_fields is None:
    #         checkbox_name_file = os.path.join(bin_dir, "global_checkbox_param.json")
    #         try:
    #             if os.path.isfile(checkbox_name_file):
    #                 with open(checkbox_name_file, "r") as fp:
    #                     self.global_checkbox_fields = json.load(fp)
    #             else:
    #                 self.global_checkbox_fields = []
    #         except Exception as exception:
    #             self.log_error(
    #                 "Get exception when loading global checkbox parameter names. "
    #                 + str(exception)
    #             )
    #             self.global_checkbox_fields = []
    #     return self.global_checkbox_fields


if __name__ == "__main__":
    exitcode = ModInputAzureCloudDefender().run(sys.argv)
    sys.exit(exitcode)
