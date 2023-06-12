import json
import os
import pathlib
import sys

import pytest

# Add bin directory for imports
bindir = os.getcwd() + "/../TA_SSPHP/bin/"
sys.path.insert(1, bindir)

from azure_client import AzureClient


def pytest_sessionstart():
    # Build fake Splunk env
    pathlib.Path("SPLUNK_HOME").mkdir(parents=True, exist_ok=True)

    os.environ["SPLUNK_HOME"] = "SPLUNK_HOME"

    pathlib.Path("SPLUNK_HOME/bin").mkdir(parents=True, exist_ok=True)

    with open("SPLUNK_HOME/bin/splunk", "w") as f:
        f.write("#! /bin/sh")

    os.chmod("SPLUNK_HOME/bin/splunk", 0o744)

    pathlib.Path("SPLUNK_HOME/var/log/splunk").mkdir(parents=True, exist_ok=True)


@pytest.fixture()
def azure_app_account():
    azure_app_account = {
        "azure_app_account": {
            "username": os.environ.get("azure_client_id"),
            "password": os.environ.get("azure_client_secret"),
        },
        "tenant_id": os.environ.get("azure_tenant_id"),
    }
    return azure_app_account


@pytest.fixture()
def az(azure_app_account):
    az = AzureClient()
    az.input_stanzas = {}
    az.input_stanzas = azure_app_account

    def get_arg(self, x):
        return self.input_stanzas.get(x, None)

    az.get_arg = get_arg.__get__(az)
    return az


@pytest.fixture
def sub_ids(az):
    subscriptions = az.get_subscriptions()
    return az.subscription_ids(subscriptions)


@pytest.fixture
def ew():
    class EventWriter:
        def __init__(self):
            self.events = []

        def write_event(self, event):
            data = json.loads(event["data"])
            assert "SSPHP_RUN" in data
            self.events.append(event)

    return EventWriter()
