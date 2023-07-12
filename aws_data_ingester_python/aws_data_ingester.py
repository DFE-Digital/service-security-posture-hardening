import logging
import os
from aws import AWS
from splunk import HecEvent, Splunk


logger = logging.getLogger("aws_data_ingester")
logging.basicConfig()
logger.setLevel(logging.DEBUG)


def log_to_splunk(splunk, event):
    hec_event = HecEvent(
        {"event": event}, sourcetype="aws_data_ingester", source="aws", host="aktest"
    )
    splunk.send_batch(hec_event.to_json())


def main():
    logger.info("Starting AWS Collection")

    index_ack_token = os.getenv("aws_splunk_token")
    splunk_hec_host = os.getenv("aws_splunk_host")
    splunk = Splunk(splunk_hec_host, index_ack_token, verify=False, indexer_ack=True)
    log_to_splunk(splunk, "Starting AWS Data Ingestion")

    aws = AWS(splunk, source="AWS", host="aktest")

    users = aws.users()
    aws.mfa()
    aws.virtual_mfa()
    aws.policies(users)
    aws.account_summary()
    aws.credential_report()
    aws.cloudtrail()
    aws.organization()

    zones = aws.route53()
    aws.resolve_dns(zones)
    log_to_splunk(splunk, f"Sending {len(splunk.queue)} events to splunk")
    logger.info("Sending events to Splunk")
    splunk.send()
    logger.info("Completed sending events to Splunk")
    log_to_splunk(splunk, f"AWS Ingestion Complete")


if __name__ == "__main__":
    main()
