#![feature(async_fn_in_trait)]
use anyhow::{Context, Result};
use clap::Parser;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::io::{self, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;

// A trait to provide functions to work with the Splunk Modular Input interface
pub trait ModularInput {
    //fn new() -> Self;
    // Print the Scheme
    fn scheme(&self) -> Result<()> {
        Ok(())
    }

    // Validate the input arguments
    async fn validate_arguments(&self, _input: &Input) -> Result<()> {
        Ok(())
    }

    // Run the input
    async fn run(&self) -> Result<()>;

    fn write_event0(&self, event: &[u8]) -> Result<()> {
        io::stdout().write_all(event)?;
        Ok(())
    }

    fn write_event_bytes(&self, event: &[u8]) -> Result<()> {
        io::stdout().write_all(event)?;
        Ok(())
    }

    fn write_event_xml<T: AsXml>(&self, event: &T) -> Result<()> {
        let out = event.as_xml()?;
        io::stdout().write_all(out.as_bytes())?;
        Ok(())
    }

    fn start_stream(&self) -> Result<()> {
        io::stdout().write_all("<stream>".as_bytes())?;
        Ok(())
    }

    fn close_stream(&self) -> Result<()> {
        io::stdout().write_all("</stream>".as_bytes())?;
        Ok(())
    }

    fn test_write_speed(&self) -> Result<()> {
        let buf = "foo";
        loop {
            self.write_event_bytes(buf.as_bytes())?;
        }
    }

    //    fn stdout(&self) -> &BufWriter<StdoutLock>;

    fn start_event_writing_thread(&self) -> (std::thread::JoinHandle<Result<()>>, Sender<Event>) {
        let (sender, receiver) = channel::<Event>();
        let join_handle = thread::spawn(move || -> Result<()> {
            io::stdout().write_all("<stream>".as_bytes())?;
            while let Ok(event) = receiver.recv() {
                let out = event.as_xml()?;
                io::stdout().write_all(out.as_bytes())?;
            }
            io::stdout().write_all("</stream>".as_bytes())?;
            Ok(())
        });
        (join_handle, sender)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    pub scheme: bool,
    #[arg(long)]
    pub validate_arguments: bool,
}

// macro_rules3

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "scheme", rename_all = "lowercase")]
pub struct Scheme {
    pub title: String,
    pub description: String,
    pub streaming_mode: String,
    pub use_single_instance: bool,
}

pub trait AsXml: Serialize {
    fn as_xml(&self) -> Result<String> {
        quick_xml::se::to_string(&self).map_err(anyhow::Error::msg)
    }
}

impl AsXml for Scheme {}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename = "event", rename_all = "lowercase")]
pub struct Event {
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sourcetype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
    #[serde(rename = "@stanza")]
    pub stanza: Option<String>,
}

impl Event {
    pub fn new() -> Self {
        Self {
            data: None,
            host: None,
            index: None,
            source: None,
            sourcetype: None,
            time: None,
            stanza: None,
        }
    }

    pub fn data_from<T: Serialize>(mut self, obj: &T) -> anyhow::Result<Self> {
        self.data = Some(serde_json::to_string(obj)?);
        Ok(self)
    }

    pub fn data(mut self, data: &str) -> Self {
        self.data = Some(data.to_owned());
        self
    }

    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.to_owned());
        self
    }

    pub fn index(mut self, index: &str) -> Self {
        self.index = Some(index.to_owned());
        self
    }
    pub fn source(mut self, source: &str) -> Self {
        self.source = Some(source.to_owned());
        self
    }

    pub fn sourcetype(mut self, sourcetype: &str) -> Self {
        self.sourcetype = Some(sourcetype.to_owned());
        self
    }

    // Might need other time formats....
    pub fn time(mut self, time: f64) -> Self {
        self.time = Some(time);
        self
    }

    pub fn stanza(mut self, stanza: &str) -> Self {
        self.stanza = Some(stanza.to_owned());
        self
    }
}

impl AsXml for Event {}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Input {
    server_host: String,
    server_uri: String,
    session_key: String,
    checkpoint_dir: String,
    configuration: InputConfiguration,
}

impl Input {
    pub fn from_xml(xml: &str) -> Result<Self> {
        from_str(xml).map_err(anyhow::Error::msg)
    }

    pub fn from_stdin() -> Result<Self> {
        if atty::isnt(atty::Stream::Stdin) {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            return Input::from_xml(&buffer)
                .with_context(|| format!("Failed to deserialize STDIN to Input: {}", buffer));
        }
        Ok(Self::default())
    }

    pub fn param_by_name(&self, name: &str) -> Option<&str> {
        for param in self.configuration.stanza.param.iter() {
            if param.name == name {
                return param.value.as_deref();
            }
        }
        None
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct InputConfiguration {
    stanza: InputStanza,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct InputStanza {
    param: Vec<InputParam>,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@app")]
    app: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct InputParam {
    //param: Vec<Param>,
    #[serde(rename = "$value")]
    value: Option<String>,
    #[serde(rename = "@name")]
    name: String,
}

#[test]
fn test_input() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<input>
  <server_host>SOMEHOST</server_host>
  <server_uri>https://127.0.0.1:8089</server_uri>
  <session_key>SOMEKEY</session_key>
  <checkpoint_dir>/opt/splunk/var/lib/splunk/modinputs/APP</checkpoint_dir>
  <configuration>
    <stanza name="github://ORG" app="APP">
      <param name="host">$decideOnStartup</param>
      <param name="index">default</param>
      <param name="interval">-1</param>
      <param name="org">ORG</param>
      <param name="github_token">TOKEN</param>
      <param name="run_introspection">true</param>
      <param name="start_by_shell">false</param>
    </stanza>
  </configuration>
</input>"#;
    let input = Input::from_xml(xml).unwrap();
    let expected = Input {
        server_host: "SOMEHOST".to_string(),
        server_uri: "https://127.0.0.1:8089".to_string(),
        session_key: "SOMEKEY".to_string(),
        checkpoint_dir: "/opt/splunk/var/lib/splunk/modinputs/APP".to_string(),
        configuration: InputConfiguration {
            stanza: InputStanza {
                param: vec![
                    InputParam {
                        name: "host".to_string(),
                        value: Some("$decideOnStartup".to_string()),
                    },
                    InputParam {
                        name: "index".to_string(),
                        value: Some("default".to_string()),
                    },
                    InputParam {
                        name: "interval".to_string(),
                        value: Some("-1".to_string()),
                    },
                    InputParam {
                        name: "org".to_string(),
                        value: Some("ORG".to_string()),
                    },
                    InputParam {
                        name: "github_token".to_string(),
                        value: Some("TOKEN".to_string()),
                    },
                    InputParam {
                        name: "run_introspection".to_string(),
                        value: Some("true".to_string()),
                    },
                    InputParam {
                        name: "start_by_shell".to_string(),
                        value: Some("false".to_string()),
                    },
                ],
                name: "github://ORG".to_string(),
                app: "APP".to_string(),
            },
        },
    };
    assert_eq!(input, expected);
}
