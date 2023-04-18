import os

import azure_defender_secure_score
import pytest


@pytest.fixture
def adss(ew):
    adss = azure_defender_secure_score.ModInputazure_defender_secure_score()
    azure_app_account = {
        "azure_app_account": {
            "username": os.environ.get("azure_client_id"),
            "password": os.environ.get("azure_client_secret"),
        },
        "tenant_id": os.environ.get("azure_tenant_id"),
        "environment": "global",
        "subscription_id": os.environ.get("subscription_id", "default_id"),
    }

    adss.input_stanzas["someapp"] = azure_app_account

    # Fake out proxy settings
    class Empty:
        pass

    adss.setup_util = Empty()
    adss.setup_util.get_proxy_settings = lambda: None
    adss.get_check_point = lambda a: None
    adss.save_check_point = lambda a, b: None
    adss.new_event = lambda data, source, index, sourcetype: {
        "data": data,
        "source": source,
        "index": index,
        "sourcetype": sourcetype,
    }
    return adss


def test_can_instantiate(adss):
    assert adss


def test_get_scheme(adss):
    scheme = adss.get_scheme()
    assert scheme


@pytest.mark.live
def test_adss_collect_events(adss, ew):
    scores = adss.collect_events(ew)
    assert scores
    for score in scores:
        assert score
        assert "SSPHP_RUN" in score
        assert "meta" in score
