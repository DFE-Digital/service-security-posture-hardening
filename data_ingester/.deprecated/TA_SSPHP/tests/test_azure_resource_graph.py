import os
from pprint import PrettyPrinter

import azure_resource_graph
import pytest
import json

PP = PrettyPrinter(indent=4, width=300, compact=False).pprint


@pytest.fixture
def arg(ew):
    arg = azure_resource_graph.ModInputazure_resource_graph()
    azure_app_account = {
        "azure_app_account": {
            "username": os.environ.get("azure_client_id"),
            "password": os.environ.get("azure_client_secret"),
        },
        "tenant_id": os.environ.get("azure_tenant_id"),
        "environment": "global",
        "collect_security_center_alerts": True,
        "collect_security_assessments": True,
        "security_assessment_sourcetype": "azure:security:assessment",
    }

    arg.input_stanzas["someapp"] = azure_app_account

    # Fake out proxy settings
    class Empty:
        pass

    arg.setup_util = Empty()
    arg.setup_util.get_proxy_settings = lambda: None
    arg.get_check_point = lambda a: None
    arg.save_check_point = lambda a, b: None

    def new_event(data=None, source=None, index=None, sourcetype=None, time=None):
        return {
            "data": data,
            "source": source,
            "index": index,
            "sourcetype": sourcetype,
            "time": time,
        }

    arg.new_event = new_event
    arg.event_writer = ew
    return arg


@pytest.mark.live
def test_collect_resource_graph(arg, ew):
    arg.collect_events(ew)
    PP(ew.events)
    assert ew.events
    assert json.loads(ew.events[0]["data"])["action"] == "start"
    assert json.loads(ew.events[-1]["data"])["action"] == "complete"
