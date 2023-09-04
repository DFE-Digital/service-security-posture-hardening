import logging
import os
from data_ingester_common.aws import AWS
from data_ingester_common.splunk import Splunk, HecEvent
from data_ingester_common.ms_graph import Azure
from azure.keyvault.secrets import SecretClient
from azure.identity import DefaultAzureCredential
import botocore

logger = logging.getLogger("data_ingester_aws")
logging.basicConfig()
logger.setLevel(logging.INFO)


def get_secrets():
    keyVaultName = os.environ["KEY_VAULT_NAME"]
    KVUri = f"https://{keyVaultName}.vault.azure.net"

    credential = DefaultAzureCredential()
    client = SecretClient(vault_url=KVUri, credential=credential)
    secret_names = [
        "splunk-token",
        "splunk-host",
        "aws-access-key-id",
        "aws-secret-access-key",
    ]
    secrets = {}
    for secret in secret_names:
        retrieved_secret = client.get_secret(secret)
        secrets[secret] = retrieved_secret.value
    return secrets


def log_to_splunk(splunk, event):
    hec_event = HecEvent(
        {"event": event},
        sourcetype="data_ingester",
        source="data_ingester_aws",
        host="test",
    )
    splunk.send_batch(hec_event.to_json())


async def main(timer):
    secrets = get_secrets()
    logger.info("Starting AWS Data Ingestion")

    splunk = Splunk(
        secrets["splunk-host"], secrets["splunk-token"], verify=True, indexer_ack=True
    )
    log_to_splunk(splunk, "Starting AWS Data Ingestion")

    aws = AWS(
        secrets["aws-access-key-id"],
        secrets["aws-secret-access-key"],
        "eu-west-2",
        splunk,
        source="AWS",
        host="aktest",
    )

    users = aws.users()
    aws.policies(users)
    try:
        aws.attached_policies(users)
    except botocore.exceptions.ClientError as e:
        log_to_splunk(splunk, f"Error while running aws.attached_policies(): {e}")
    aws.mfa()
    aws.virtual_mfa()
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
    log_to_splunk(splunk, f"AWS Data Ingestion Complete")


if __name__ == "__main__":
    main(None)
