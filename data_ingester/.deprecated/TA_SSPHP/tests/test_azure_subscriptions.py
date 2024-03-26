import os

import azure_subscription
import pytest


@pytest.fixture
def a_s():
    a_s = azure_subscription.ModInputazure_subscription()
    azure_app_account = {
        "azure_app_account": {
            "username": os.environ.get("azure_client_id"),
            "password": os.environ.get("azure_client_secret"),
        },
        "tenant_id": os.environ.get("azure_tenant_id"),
        "environment": "global",
        "subscription_id": os.environ.get("subscription_id", "default_id"),
    }

    a_s.input_stanzas["someapp"] = azure_app_account

    # Fake out proxy settings
    class Empty:
        pass

    a_s.setup_util = Empty()
    a_s.setup_util.get_proxy_settings = lambda: None
    a_s.get_check_point = lambda a: None
    a_s.save_check_point = lambda a, b: None
    a_s.new_event = lambda data, source, index, sourcetype: {
        "data": data,
        "source": source,
        "index": index,
        "sourcetype": sourcetype,
    }
    return a_s


def test_can_instantiate(a_s):
    assert a_s


def test_get_scheme(a_s):
    scheme = a_s.get_scheme()
    assert scheme


@pytest.mark.live
def test_arg_collect_events(a_s, ew):
    events = a_s.collect_events(ew)
    assert events
