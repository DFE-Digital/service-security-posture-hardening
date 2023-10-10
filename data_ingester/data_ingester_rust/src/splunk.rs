use itertools::Itertools;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
//use futures::lock::Mutex;
use anyhow::{anyhow, Result};

// TODO Should not be shared between calls to the web endpoint!
static SSPHP_RUN: RwLock<f64> = RwLock::new(0_f64);

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

pub fn set_ssphp_run() -> Result<()> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?;

    *SSPHP_RUN.write().unwrap() = since_the_epoch.as_secs_f64();
    Ok(())
}

impl HecEvent {
    // TODO: Should return Result
    pub fn new<T: Serialize>(event: &T, source: &str, sourcetype: &str) -> Result<HecEvent> {
        let ssphp_run = *SSPHP_RUN.read().unwrap();
        let ssphp_event = SsphpEvent { ssphp_run, event };
        let hostname = hostname::get()
            .unwrap()
            .into_string()
            .unwrap_or("NO HOSTNAME".to_owned());
        Ok(HecEvent {
            source: source.to_string(),
            sourcetype: sourcetype.to_string(),
            host: hostname,
            event: serde_json::to_string(&ssphp_event).unwrap(),
        })
    }
}

// #[derive(Debug, Serialize, Deserialize, Default)]
// struct Dummy {
// }

// pub trait ToHecEvents {
//     fn source() -> &'static str;
//     fn sourcetype() -> &'static str;
//     fn to_hec_events(&self) -> Result<HecEvent, Box<dyn Error + Send + Sync>> {
//         let d = Dummy {};
//         HecEvent::new(&d, "a", "b")
//     }
// }
// struct Logs {}

// fn logger() {}

pub fn to_hec_events<T: Serialize>(
    collection: &[T],
    source: &str,
    sourcetype: &str,
) -> Result<Vec<HecEvent>> {
    let (ok, err): (Vec<_>, Vec<_>) = collection
        .iter()
        .map(|u| HecEvent::new(u, source, sourcetype))
        .partition_result();
    if !err.is_empty() {
        return Err(anyhow!(err
            .iter()
            .map(|err| format!("{:?}", err))
            .collect::<Vec<String>>()
            .join("\n")));
    }
    Ok(ok)
}

pub trait ToHecEvents<'a> {
    type Item: Serialize;
    fn to_hec_events(&'a self) -> Result<Vec<HecEvent>> {
        let (ok, err): (Vec<_>, Vec<_>) = self
            .collection()
            .map(|u| HecEvent::new(&u, Self::source(), Self::sourcetype()))
            .partition_result();
        if !err.is_empty() {
            return Err(anyhow!(err
                .iter()
                .map(|err| format!("{:?}", err))
                .collect::<Vec<String>>()
                .join("\n")));
        }
        Ok(ok)
    }
    fn source() -> &'static str;
    fn sourcetype() -> &'static str;
    fn collection(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a>;
}

pub trait ToHecEvent: Serialize + Sized {
    fn to_hec_event(&self) -> Result<HecEvent> {
        HecEvent::new(self, Self::source(), Self::sourcetype())
    }
    fn source() -> &'static str;
    fn sourcetype() -> &'static str;
}

#[derive(Debug, Clone)]
pub struct Splunk {
    client: Client,
    url: String,
}

unsafe impl Send for Splunk {}
unsafe impl Sync for Splunk {}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Message {
    event: String,
}

impl Splunk {
    pub fn new(host: &str, token: &str) -> Result<Self> {
        let url = format!("https://{}/services/collector", host);
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(Splunk::headers(token)?)
            .build()?;
        Ok(Self { client, url })
    }

    fn headers(token: &str) -> Result<HeaderMap> {
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

    // TODO enable token acknowledgement
    pub async fn send_batch(&self, events: &[HecEvent]) -> Result<()> {
        for batch in events.iter().batching(batch_lines) {
            let request = self.client.post(&self.url).body(batch).build().unwrap();
            let _response = self.client.execute(request).await.unwrap();
        }
        Ok(())
    }

    pub async fn log(&self, message: &str) -> Result<()> {
        eprintln!("{}", &message);
        self.send_batch(&[HecEvent::new(
            &Message {
                event: message.to_owned(),
            },
            "data_ingester_rust",
            "data_ingester_rust_logs",
        )?])
        .await?;
        Ok(())
    }
}

// Needs Splunk Creds
#[ignore]
#[tokio::test]
async fn send_to_splunk() {
    let splunk = Splunk::new("", "").unwrap();
    let data = std::collections::HashMap::from([("aktest", "fromrust")]);
    let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
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
    let he = HecEvent::new(&data, "msgraph_rust", "test_event").unwrap();
    events.push(he);

    let data1 = HashMap::from([("aktest1", "fromrust")]);
    let he1 = HecEvent::new(&data1, "msgraph_rust", "test_event").unwrap();
    events.push(he1);
    splunk.send_batch(&events[..]).await.unwrap();
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
