import concurrent.futures
import json
import os
import sys
from datetime import datetime
import import_declare_test
from msrestazure.tools import parse_resource_id

from splunklib import modularinput as smi
from splunktaucclib.modinput_wrapper import base_modinput as base_mi

from azure_client import AzureClient

bin_dir = os.path.basename(__file__)


class ModInputazure_defender_secure_score(AzureClient, base_mi.BaseModInput):
    def __init__(self):
        use_single_instance = False
        super(ModInputazure_defender_secure_score, self).__init__(
            "ta_ms_aad", "azure_defender_secure_score", use_single_instance
        )
        self.global_checkbox_fields = None
        self.ssphp_run = datetime.now().timestamp()
        self.session = None

    def get_scheme(self):
        """overloaded splunklib modularinput method"""
        scheme = super(ModInputazure_defender_secure_score, self).get_scheme()
        scheme.title = "Azure Defender Secure Score"
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
        return scheme

    def get_app_name(self):
        return "TA-MS-AAD"

    def validate_input(helper, definition):
        pass

    def tenant_id(self):
        self.get_arg("tenant_id")

    def collect_events(self, event_writer):
        subscriptions = self.get_subscriptions()

        results = []
        executor = concurrent.futures.ThreadPoolExecutor(max_workers=5)
        for subscription_id in self.subscription_ids(subscriptions):
            results.append(executor.submit(self.list_secure_scores, subscription_id))

        metadata = {
            "sourcetype": "azure:security:score",
            "index": self.get_output_index(),
            "source": f"{self.input_type}",
        }

        count = 0

        events = []

        for r in concurrent.futures.as_completed(results):
            r = r.result()
            for alert in r:
                event = alert.serialize(keep_readonly=True)
                event["SSPHP_RUN"] = self.ssphp_run
                event.setdefault("meta", {}).update(
                    {"id": parse_resource_id(event["id"])}
                )
                event1 = self.new_event(
                    data=json.dumps(event, sort_keys=True),
                    source=metadata["source"],
                    index=metadata["index"],
                    sourcetype=metadata["sourcetype"],
                )

                event_writer.write_event(event1)
                count += 1
                events.append(event)

        sys.stdout.flush()
        self.logger.info(f"Finished writing events: {count}")
        return events

    def resource_groups_metadata(self, subscription_id):
        return {
            "sourcetype": self.get_arg("source_type"),
            "index": self.get_output_index(),
            "source": f"{self.input_type}:subscription:{subscription_id}",
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
    exitcode = ModInputazure_defender_secure_score().run(sys.argv)
    sys.exit(exitcode)
