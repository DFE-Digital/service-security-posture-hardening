from pprint import PrettyPrinter

import pytest

PP = PrettyPrinter(indent=4, width=300, compact=False).pprint


@pytest.mark.live
def test_get_azure_credentials(az):
    creds = az.get_azure_credentials()
    assert creds


@pytest.mark.live
def test_get_subscriptions(az):
    subscriptions = list(az.get_subscriptions())
    assert subscriptions
    for subscription in subscriptions:
        assert subscription.tenant_id


@pytest.mark.live
def test_get_assessments(az, sub_ids):
    for sub_id in sub_ids:
        assessments = list(az.get_assessments(sub_id))
        for assessment in assessments:
            assert assessment.type == "Microsoft.Security/assessments"


@pytest.mark.live
def test_get_assessments_metadata(az, sub_ids):
    for sub_id in sub_ids:
        assessments_metadata = list(az.get_assessment_metadata(sub_id))
        for assessment_metadata in assessments_metadata:
            assert assessment_metadata.type == "Microsoft.Security/assessmentMetadata"


@pytest.mark.live
def test_get_assessment_metadata(az, sub_ids):
    for sub_id in sub_ids:
        assessment_metadata = az.get_assessment_metadata(sub_id)
        assert assessment_metadata


@pytest.mark.skip(reason="Azure API response doesn't match documentation")
def test_get_contacts(acd, sub_ids):
    for sub_id in sub_ids:
        contacts = acd.get_contacts(sub_id)
        assert contacts


@pytest.mark.live
def test_arg_get_resource_groups(az, ew, sub_ids):
    for sub_id in sub_ids:
        rgs = az.get_resource_groups(sub_id)
        assert rgs


# @pytest.mark.live
# def test_list_alerts(az, ew, sub_ids):
#     for sub_id in sub_ids:
#         alerts = list(az.list_alerts(sub_id))
#         assert alerts
#         for alert in alerts:
#             assert alert


@pytest.mark.live
def test_list_secure_scores(az, ew, sub_ids):
    for sub_id in sub_ids:
        scores = list(az.list_secure_scores(sub_id))
        assert scores
        for score in scores:
            print(score.serialize(keep_readonly=True))
            assert score


@pytest.mark.live
def test_list_secure_score_control_definitions(az, ew, sub_ids):
    for sub_id in sub_ids:
        ssds = list(az.list_secure_score_control_definitions(sub_id))
        assert ssds
        for ssd in ssds:
            s = ssd.serialize(keep_readonly=True)
            # del(s['properties']['assessmentDefinitions'])
            PP(s)
            assert ssd


@pytest.mark.live
def test_list_secure_score_controls(az, ew, sub_ids):
    for sub_id in sub_ids:
        sscs = list(az.list_secure_score_controls(sub_id))
        assert sscs
        for ssc in sscs:
            PP(ssc.serialize(keep_readonly=True))
            assert ssc


@pytest.mark.live
def test_list_settings(az, ew, sub_ids):
    for sub_id in sub_ids:
        settings = list(az.list_settings(sub_id))
        assert settings
        for setting in settings:
            print(setting.serialize(keep_readonly=True))
            assert setting




@pytest.mark.live
def test_get_resource_graph(az, ew):
    rgs = az.get_resource_graph()

    for rg in rgs:
        print("***********************")
        PP(rg)

    assert rgs