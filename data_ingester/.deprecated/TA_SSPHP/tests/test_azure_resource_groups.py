import os

import azure_resource_group
import pytest


@pytest.fixture
def arg():
    arg = azure_resource_group.ModInputazure_resource_group()
    azure_app_account = {
        "azure_app_account": {
            "username": os.environ.get("azure_client_id"),
            "password": os.environ.get("azure_client_secret"),
        },
        "tenant_id": os.environ.get("azure_tenant_id"),
        "environment": "global",
        "subscription_id": os.environ.get("subscription_id", "default_id"),
    }

    arg.input_stanzas["someapp"] = azure_app_account

    # Fake out proxy settings
    class Empty:
        pass

    arg.setup_util = Empty()
    arg.setup_util.get_proxy_settings = lambda: None
    arg.get_check_point = lambda a: None
    arg.save_check_point = lambda a, b: None
    arg.new_event = lambda data, source, index, sourcetype: {
        "data": data,
        "source": source,
        "index": index,
        "sourcetype": sourcetype,
    }
    return arg


def test_can_instantiate(arg):
    assert arg


def test_get_scheme(arg):
    scheme = arg.get_scheme()
    assert scheme


@pytest.mark.live
def test_arg_collect_events(arg, ew):
    events = arg.collect_events(ew)
    assert events
