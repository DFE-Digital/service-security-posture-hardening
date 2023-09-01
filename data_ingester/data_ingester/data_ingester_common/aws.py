import logging
import time as pytime
import sys
import os
import boto3
import dns.resolver
from .splunk import HecEvent

logger = logging.getLogger("data_ingester_aws")
logging.basicConfig()
logger.setLevel(logging.INFO)


class AWS:
    def __init__(
            self,
            aws_access_key_id,
            aws_secret_access_key,
            region_name,
            splunk,
            source=None,
            sourcetype=None,
            host=None,
    ):
        self.splunk = splunk
        self.session = boto3.Session(
            aws_access_key_id=aws_access_key_id,
            aws_secret_access_key=aws_secret_access_key,
            region_name=region_name
        )
        self.iam = self.session.client("iam")
        self.source = source
        self.sourcetype = sourcetype
        self.host = host

    def add_to_splunk(self, collection, source=None, sourcetype=None, host=None):
        if not source:
            source = self.source
        if not sourcetype:
            sourcetype = self.sourcetype
        if not host:
            host = self.host

        if not isinstance(collection, list):
            collection = [collection]

        hec_events = HecEvent.events(
            collection, source=source, sourcetype=sourcetype, host=host
        )
        self.splunk.extend(hec_events)

    def users(self):
        logger.info("Getting iam.list_users")
        users = self.iam.list_users()["Users"]
        for user in users:
            tags = self.iam.list_user_tags(UserName=user["UserName"])
            del tags["IsTruncated"]
            del tags["ResponseMetadata"]
            user["tags"] = tags
        self.add_to_splunk(users, sourcetype="list_users")
        return users

    def mfa(self):
        logger.info("Getting iam.list_mfa_devices")
        mfa = self.iam.list_mfa_devices()["MFADevices"]
        self.add_to_splunk(mfa, sourcetype="list_mfa_devices")
        return mfa

    def virtual_mfa(self):
        logger.info("Getting iam.list_virtual_mfa_devices")
        virtual_mfa = self.iam.list_virtual_mfa_devices()["VirtualMFADevices"]
        self.add_to_splunk(virtual_mfa, sourcetype="list_virtual_mfa_devices")
        return virtual_mfa

    def policies(self, users):
        logger.info("Getting iam.list_user_policies")
        policies = []
        for user in users:
            policy_names = self.iam.list_user_policies(UserName=user["UserName"])[
                "PolicyNames"
            ]
            for policy_name in policy_names:
                policy = self.iam.get_user_policy(
                    UserName=user["UserName"], PolicyName=policy_name
                )
                if not policy:
                    continue
                del policy["ResponseMetadata"]
                policy["UserArn"] = user["Arn"]
                policies.append(policy)

        self.add_to_splunk(policies, sourcetype="get_user_policy")
        return policies

    def attached_policies(self, users):
        logger.info("Getting iam.list_attached_user_policies")
        attached_policies = []
        policies = []
        for user in users:
            attached_policies = self.iam.list_attached_user_policies(
                UserName=user["UserName"]
            )
            attached_policies["UserArn"] = user["Arn"]
            del attached_policies["ResponseMetadata"]

            for attached_policy in attached_policies["AttachedPolicies"]:
                policy = self.iam.get_policy(PolicyArn=attached_policy["PolicyArn"])[
                    "Policy"
                ]
                policies.append(policy)

        self.add_to_splunk(attached_policies, sourcetype="get_policy")
        self.add_to_splunk(policies, sourcetype="get_attached_user_policy")

        return (attached_policies, policies)

    def credential_report(self):
        logger.info("Getting iam.get_credential_report")
        while True:
            report = self.iam.generate_credential_report()
            if report["State"] == "COMPLETE":
                break
            pytime.sleep(1)

        credential_report = self.iam.get_credential_report()
        del credential_report["ResponseMetadata"]
        self.add_to_splunk(credential_report, sourcetype="get_credential_report")
        return credential_report

    def account_summary(self):
        logger.info("Getting iam.get_account_summary")
        account_summary = self.iam.get_account_summary()["SummaryMap"]
        self.add_to_splunk(account_summary, sourcetype="get_account_summary")

    def cloudtrail(self):
        logger.info("Getting iam.list_trails")
        ct_client = self.session.client("cloudtrail")
        trails = ct_client.list_trails()["Trails"]
        cloudtrails = []
        for trail in trails:
            cloudtrails.append(ct_client.get_trail(Name=trail["Name"])["Trail"])
        self.add_to_splunk(cloudtrails, sourcetype="get_trail")
        return cloudtrails

    def organization(self):
        logger.info("Getting organizations.describe_organization")
        try:
            org = self.session.client("organizations").describe_organization()[
                "Organization"
            ]
        except:
            account_id = self.session.client("sts").get_caller_identity().get("Account")
            org = {"InOrganization": False, "AccountId": account_id}
        self.add_to_splunk(org, sourcetype="describe_organization")
        return org

    def route53(self):
        r53 = self.session.client("route53")
        hosted_zones = r53.list_hosted_zones()["HostedZones"]
        zones = []
        for hosted_zone in hosted_zones:
            zone = r53.get_hosted_zone(Id=hosted_zone["Id"])
            del zone["ResponseMetadata"]
            zone["ResourceRecordSets"] = r53.list_resource_record_sets(
                HostedZoneId=hosted_zone["Id"]
            )["ResourceRecordSets"]

            zones.append(zone)

        self.add_to_splunk(zones, sourcetype="hosted_zones")
        return zones

    def resolve_dns(self, zones):
        record_sets = []
        for zone in zones:
            for resource_record_set in zone["ResourceRecordSets"]:

                name = resource_record_set["Name"]
                record_type = resource_record_set["Type"]
                resource_records = [
                    rr["Value"] for rr in resource_record_set["ResourceRecords"]
                ]
                try:
                    nx_domain_error = None
                    answers = dns.resolver.resolve(name, record_type)
                except dns.resolver.NXDOMAIN as e:
                    answers = []
                    nx_domain_error = e
                rrs = {
                    "Name": name,
                    "Type": record_type,
                    "ResourceRecords": [],
                    "HostedZone": {"Name": zone["HostedZone"]["Name"]},
                }

                if nx_domain_error:
                    rrs["error"] = nx_domain_error

                for rdata in answers:
                    rrs["ResourceRecords"].append(
                        {
                            "Value": str(rdata),
                            "InRoute53": str(rdata) in resource_records,
                        }
                    )
                    record_sets.append(rrs)
        self.add_to_splunk(record_sets, sourcetype="hosted_zones_from_dns")
        return record_sets
