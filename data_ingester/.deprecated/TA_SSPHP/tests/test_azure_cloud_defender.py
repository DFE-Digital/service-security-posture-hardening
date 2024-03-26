import os
from pprint import PrettyPrinter

import azure_cloud_defender
import pytest
from azure.mgmt.security.v2021_06_01.models import SecurityAssessmentResponse

PP = PrettyPrinter(indent=4, width=300, compact=False).pprint


@pytest.fixture
def acd(ew):
    acd = azure_cloud_defender.ModInputAzureCloudDefender()
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

    acd.input_stanzas["someapp"] = azure_app_account

    # Fake out proxy settings
    class Empty:
        pass

    acd.setup_util = Empty()
    acd.setup_util.get_proxy_settings = lambda: None
    acd.get_check_point = lambda a: None
    acd.save_check_point = lambda a, b: None
    acd.new_event = lambda data, source, index, sourcetype: {
        "data": data,
        "source": source,
        "index": index,
        "sourcetype": sourcetype,
    }
    acd.event_writer = ew
    return acd


@pytest.fixture
def sar():
    sar = SecurityAssessmentResponse()
    sar.additional_data = {}
    sar.additional_data.update(
        {
            "subAssessmentsLink": "/subscriptions/63ed7111-101c-4849-9f33-03ef672ed20d/providers/Microsoft.Security/assessments/fde1c0c9-0fd2-4ecc-87b5-98956cbc1095/subAssessments"
        }
    )
    return sar


def test_extract_assessment_resource_scope(sar):
    scope = sar.sub_assessment_resource_scope()
    assert scope == "/subscriptions/63ed7111-101c-4849-9f33-03ef672ed20d"


# Base Class
def test_can_instantiate(acd):
    acd = azure_cloud_defender.ModInputAzureCloudDefender()
    assert acd


def test_get_scheme(acd):
    scheme = acd.get_scheme()
    assert scheme


@pytest.mark.live
def test_collect_events(acd, ew):
    events = acd.collect_events(ew)
    assert events


@pytest.mark.live
def test_get_sub_assessment_fail(acd, sar, sub_ids):
    assessments = list(acd.get_assessments(sub_ids[0]))
    events = acd.get_sub_assessments(assessments[0])
    assert 0 == len(events)


@pytest.mark.live
def test_get_events_threaded(acd, ew):
    events = acd.get_events_threaded()
    assert events


@pytest.mark.live
def test_get_subscription_events(acd, ew, sub_ids):
    for sub_id in sub_ids:
        events = list(acd.get_subscription_events(sub_id))
        for event in events:
            metadata = event.get("assessment_metadata", {})
            if metadata:
                sscd = metadata.get("secure_score_control_details")
                assert sscd
