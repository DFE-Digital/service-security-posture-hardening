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
import os
import sys
from datetime import datetime
import json
import import_declare_test

from splunklib import modularinput as smi
from splunktaucclib.modinput_wrapper import base_modinput as base_mi
from azure_client import AzureClient

bin_dir = os.path.basename(__file__)


class ModInputazure_resource_graph(AzureClient, base_mi.BaseModInput):
    def __init__(self):
        use_single_instance = False
        super(ModInputazure_resource_graph, self).__init__(
            "ta_ssphp", "azure_resource_graph", use_single_instance
        )
        self.global_checkbox_fields = None
        self.ssphp_run = datetime.now().timestamp()

    def get_scheme(self):
        """overloaded splunklib modularinput method"""
        scheme = super(ModInputazure_resource_graph, self).get_scheme()
        scheme.title = "Azure Resource Graphs"
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
        scheme.add_argument(
            smi.Argument(
                "environment",
                title="Environment",
                description="",
                required_on_create=True,
                required_on_edit=False,
            )
        )
        scheme.add_argument(
            smi.Argument(
                "source_type",
                title="Resource Graph Sourcetype",
                description="",
                required_on_create=True,
                required_on_edit=False,
            )
        )
        return scheme

    def get_app_name(self):
        return "TA_SSPHP"

    def validate_input(helper, definition):
        pass

    def write_events(self, event_writer, collection, metadata):
        """Write a collection of events using the provided eventwriter and metadata"""
        for table, array in collection.items():
            for data in array:
                data["SSPHP_RUN"] = self.ssphp_run
                event = self.new_event(
                    data=json.dumps(data),
                    time=datetime.now().timestamp(),
                    source=f"{metadata['source']}:{table}",
                    index=metadata["index"],
                    sourcetype=metadata["sourcetype"],
                )
                event_writer.write_event(event)
            sys.stdout.flush()

    def start_event(self, event_writer):
        data = {
            "SSPHP_RUN": self.ssphp_run,
            "action": "start",
            "input": "azure_resource_graph",
            "app": "TA_SSPHP",
        }
        event = self.new_event(
            data=json.dumps(data),
            time=datetime.now().timestamp(),
            sourcetype=f"{self.get_arg('source_type')}:SSPHP_RUN",
            index=self.get_output_index(),
            source=f"{self.input_type}:resource_graph",
        )
        event_writer.write_event(event)
        sys.stdout.flush()

    def complete_event(self, event_writer, subscription_count, event_count, details):
        data = {
            "SSPHP_RUN": self.ssphp_run,
            "SSPHP_RUN_COMPLETE": datetime.now().timestamp(),
            "action": "complete",
            "input": "azure_resource_graph",
            "app": "TA_SSPHP",
            "events": event_count,
            "subscriptions": subscription_count,
            "details": details,
        }
        event = self.new_event(
            data=json.dumps(data),
            time=datetime.now().timestamp(),
            sourcetype=f"{self.get_arg('source_type')}:SSPHP_RUN",
            index=self.get_output_index(),
            source=f"{self.input_type}:resource_graph",
        )
        event_writer.write_event(event)
        sys.stdout.flush()

    def collect_events(self, event_writer):
        self.start_event(event_writer)

        subscriptions = self.get_subscriptions()

        stats_subscription_count = 0
        stats_event_count = 0
        stats_details = {}
        for subscription_id in self.subscription_ids(subscriptions):
            resource_graphs = self.get_resource_graph(subscription_id, event_writer)

            self.write_events(
                event_writer,
                resource_graphs,
                self.resource_graph_metadata(),
            )

            stats_subscription_count += 1

            for table, events in resource_graphs.items():
                event_count = len(events)
                stats_event_count += event_count
                stats_details.setdefault(subscription_id, {}).setdefault(
                    table, event_count
                )

        self.complete_event(
            event_writer, stats_subscription_count, stats_event_count, stats_details
        )
        return resource_graphs

    def resource_graph_metadata(self):
        return {
            "sourcetype": self.get_arg("source_type"),
            "index": self.get_output_index(),
            "source": f"{self.input_type}:resource_graph",
        }

    def get_account_fields(self):
        account_fields = []
        account_fields.append("azure_app_account")
        return account_fields

    def get_checkbox_fields(self):
        checkbox_fields = []
        return checkbox_fields

    def get_global_checkbox_fields(self):
        if self.global_checkbox_fields is None:
            checkbox_name_file = os.path.join(bin_dir, "global_checkbox_param.json")
            try:
                if os.path.isfile(checkbox_name_file):
                    with open(checkbox_name_file, "r") as fp:
                        self.global_checkbox_fields = json.load(fp)
                else:
                    self.global_checkbox_fields = []
            except Exception as e:
                self.log_error(
                    "Get exception when loading global checkbox parameter names. "
                    + str(e)
                )
                self.global_checkbox_fields = []
        return self.global_checkbox_fields


if __name__ == "__main__":
    exitcode = ModInputazure_resource_graph().run(sys.argv)
    sys.exit(exitcode)
