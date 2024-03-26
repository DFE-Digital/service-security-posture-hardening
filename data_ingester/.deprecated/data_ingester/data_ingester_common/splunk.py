import logging
import json
import time as pytime
import uuid
import requests
from .ssphp import SSPHP_RUN

logger = logging.getLogger("data_ingester_splunk")
logging.basicConfig()
logger.setLevel(logging.INFO)


class Splunk:
    def __init__(self, host, token, verify=True, indexer_ack=False):
        self.queue = []
        self.token = token
        self.host = host
        self.verify = verify
        self.channel = uuid.uuid4()
        self.indexer_ack = indexer_ack
        self.batches = {}

    def url(self):
        return f"https://{self.host}/services/collector"

    def headers(self):
        headers = {"Authorization": f"Splunk {self.token}"}
        if self.indexer_ack:
            headers["X-Splunk-Request-Channel"] = str(self.channel)
        return headers

    def append(self, event):
        self.queue.append(event)

    def extend(self, events):
        self.queue.extend(events)

    def batch(self):
        batch = []
        batches = []
        batch_size = 0
        batch_size_limit = 900_000
        batch_len_limit = 90
        for event in self.queue:
            json_event = event.to_json()
            batch_size += len(json_event)
            if batch_size > batch_size_limit or len(batch) > batch_len_limit:
                batches.append("\n".join(batch))
                batch = []
                batch_size = 0
            batch.append(json_event)
        batches.append("\n".join(batch))
        return batches

    def send(self):
        if not self.queue:
            logger.info("Splunk queue empty")            
            return
        logger.info("Sending batches to Splunk")
        for batch in self.batch():
            response = requests.post(
                self.url(),
                headers=self.headers(),
                data=batch,
                verify=self.verify,
                timeout=20,
            )
            if self.indexer_ack:
                logger.info(response.json())
                ack_id = response.json()["ackId"]
                self.batches[ack_id] = batch

        if self.indexer_ack:
            self.check_acks()

    def send_batch(self, batch):
        logger.info("Sending single batch")
        batches = {}
        response = requests.post(
            self.url(),
            headers=self.headers(),
            data=batch,
            verify=self.verify,
            timeout=20,
        )
        ack_id = response.json()["ackId"]
        self.batches[ack_id] = batch

    def resend_batches(self):
        logger.warning("Resending %s batches" % str(len(self.batches)))
        batches = {}
        for ack_id, batch in self.batches.items():
            response = requests.post(
                self.url(),
                headers=self.headers(),
                data=batch,
                verify=self.verify,
                timeout=20,
            )
            ack_id = response.json()["ackId"]
            batches[ack_id] = batch
        self.batches = batches

    def check_acks(self):
        url = f"https://{self.host}/services/collector/ack"
        count = 0
        while True:
            j = json.dumps({"acks": list(self.batches.keys())})
            response = requests.post(
                url, headers=self.headers(), verify=self.verify, timeout=20, data=j
            )
            for ack_id, status in response.json()["acks"].items():
                if status:
                    del self.batches[int(ack_id)]
            if self.batches:
                pytime.sleep(10)
            else:
                break
            if count >= 6:
                self.resend_batches()
                count = 0
            count += 1


class HecEvent:
    def __init__(
        self, event, sourcetype=None, source=None, host=None, time=None, metadata=None
    ):
        self.event = event
        self.sourcetype = sourcetype
        self.source = source
        self.host = host
        self.time = time
        self.metadata = metadata

    def event_to_json(self):
        self.event["SSPHP_RUN"] = SSPHP_RUN
        return json.dumps(self.event, default=str)

    def to_json(self):
        hec_event = {
            "event": self.event_to_json(),
        }
        if self.source:
            hec_event["source"] = self.source

        if self.sourcetype:
            hec_event["sourcetype"] = self.sourcetype

        if self.host:
            hec_event["host"] = self.host

        if self.time:
            hec_event["time"] = self.time

        if self.metadata:
            hec_event["metadata"] = self.metadata

        return json.dumps(hec_event, default=str)

    @staticmethod
    def events(
        events, sourcetype=None, source=None, host=None, time=None, metadata=None
    ):
        return [
            HecEvent(
                event,
                sourcetype=sourcetype,
                source=source,
                host=host,
                time=time,
                metadata=metadata,
            )
            for event in events
        ]
