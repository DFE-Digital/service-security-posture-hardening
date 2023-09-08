use itertools::Itertools;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

static SSPHP_RUN: OnceLock<f64> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct HecEvent {
    source: String,
    sourcetype: String,
    host: String,
    event: String,
}

#[derive(Serialize, Deserialize)]
struct SsphpEvent<T> {
    #[serde(rename = "SSPHP_RUN")]
    ssphp_run: f64,
    #[serde(flatten)]
    event: T,
}

impl HecEvent {
    pub fn new<T: Serialize>(event: &T, source: &str, sourcetype: &str) -> Self {
        let ssphp_run = *SSPHP_RUN.get_or_init(|| {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            since_the_epoch.as_secs_f64()
        });

        let ssphp_event = SsphpEvent { ssphp_run, event };
        let hostname = hostname::get().unwrap().into_string().unwrap();
        HecEvent {
            source: source.to_string(),
            sourcetype: sourcetype.to_string(),
            host: hostname,
            event: serde_json::to_string(&ssphp_event).unwrap(),
        }
    }
}

pub struct Splunk {
    client: Client,
    url: String,
}

impl Splunk {
    pub fn new(host: &str, token: &str) -> Result<Self, Box<dyn Error>> {
        let url = format!("https://{}/services/collector", host);
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(Splunk::headers(token)?)
            .build()?;
        Ok(Self { client, url })
    }

    fn headers(token: &str) -> Result<HeaderMap, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Splunk {}", token))?;
        auth.set_sensitive(true);
        headers.insert("Authorization", auth);
        let channel = Uuid::new_v4().to_string();
        headers.insert("X-Splunk-Request-Channel", channel.parse()?);
        Ok(headers)
    }

    pub async fn send(&self, event: &HecEvent) {
        let request = self.client.post(&self.url).json(event).build().unwrap();
        let _response = self.client.execute(request).await.unwrap();
    }

    pub async fn send_batch(&self, events: &[HecEvent]) {
        for batch in events.iter().batching(batch_lines) {
            let request = self.client.post(&self.url).body(batch).build().unwrap();
            let _response = self.client.execute(request).await.unwrap();
        }
    }
}

// Needs Splunk Creds
#[ignore]
#[tokio::test]
async fn send_to_splunk() {
    let splunk = Splunk::new("", "").unwrap();
    let data = std::collections::HashMap::from([("aktest", "fromrust")]);
    let he = HecEvent::new(&data, "msgraph_rust", "test_event");
    splunk.send(&he).await;
}

// Needs Splunk Creds
#[ignore]
#[tokio::test]
async fn send_batch_to_splunk() {
    use std::collections::HashMap;
    let splunk = Splunk::new("", "").unwrap();
    let mut events = Vec::new();
    let data = HashMap::from([("aktest0", "fromrust")]);
    let he = HecEvent::new(&data, "msgraph_rust", "test_event");
    events.push(he);

    let data1 = HashMap::from([("aktest1", "fromrust")]);
    let he1 = HecEvent::new(&data1, "msgraph_rust", "test_event");
    events.push(he1);
    splunk.send_batch(&events).await;
}

fn batch_lines<I, T: Serialize>(it: &mut I) -> Option<String>
where
    I: Iterator<Item = T>,
    I: std::fmt::Debug,
{
    let max = 1024 * 950;
    let mut lines = String::with_capacity(max);

    let mut size = 0;
    while size < max {
        match it.next() {
            None => {
                break;
            }
            Some(x) => {
                //                let s = x.unwrap();
                let json = serde_json::to_string(&x).unwrap();
                size += json.len();
                lines.push_str(json.as_str());
                lines.push('\n');
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}
