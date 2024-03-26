
import logging
logger = logging.getLogger("data_ingester_aad_ms_graph")
logging.basicConfig()
logger.setLevel(logging.INFO)
import sys
logger.info("Starting AAD MS Graph Collection")
logger.info(sys.version)
print(sys.version)
import os
from data_ingester_common.splunk import Splunk, HecEvent
from data_ingester_common.ms_graph import Azure
from azure.keyvault.secrets import SecretClient
from azure.identity import DefaultAzureCredential



def get_secrets():
    keyVaultName = os.environ["KEY_VAULT_NAME"]
    KVUri = f"https://{keyVaultName}.vault.azure.net"

    credential = DefaultAzureCredential()
    client = SecretClient(vault_url=KVUri, credential=credential)
    secret_names = [
        "ad-client-id",
        "ad-client-secret",
        "ad-tenant-id",
        "splunk-token",
        "splunk-host",
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
        source="data_ingester_aad_ms_graph",
        host="test",
    )
    splunk.send_batch(hec_event.to_json())


async def main(timer):
    logger.info("running")
    logger.info(sys.version)
    print(sys.version)

    secrets = get_secrets()
    logger.info("Starting AAD MS Graph Collection")

    splunk = Splunk(
        secrets["splunk-host"], secrets["splunk-token"], verify=True, indexer_ack=True
    )
    log_to_splunk(splunk, "Starting MS Graph Data Ingestion")

    azure = Azure(
        splunk,
        source="Azure",
        host="test",
        tenant_id=secrets["ad-tenant-id"],
        client_id=secrets["ad-client-id"],
        client_secret=secrets["ad-client-secret"],
    )

    await azure.run()

    splunk.send()


if __name__ == "__main__":
    main(None)
