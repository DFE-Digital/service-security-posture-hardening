use std::env;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::Mutex;

use bytes::Bytes;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use warp::{http::Response, Filter};

use crate::keyvault::get_keyvault_secrets;
use crate::ms_graph::azure;
use crate::ms_graph::m365;
use crate::powershell::install_powershell;
use crate::splunk::set_ssphp_run;
use crate::splunk::Splunk;
use anyhow::Result;

// Request headers
// {
//     "host": "127.0.0.1:34963",
//     "x-azure-functions-hostversion": "4.24.4.4",
//     "x-azure-functions-invocationid": "83a33220-a921-460a-bec0-b3f043dcf1ff",
//     "user-agent": "Azure-Functions-Host/4.24.4.4",
//     "transfer-encoding": "chunked",
//     "traceparent": "00-bb2bd5eef54cb15a1506d0326d2489e7-d3f54c84c7034d76-00",
//     "content-type": "application/json; charset=utf-8"
// }

// Timer payload
// {
//     "Data": {
//         "timer": {
//             "Schedule":{
//                 "AdjustForDST":true
//             },
//             "ScheduleStatus":null,
//             "IsPastDue":false
//         }
//     },
//     "Metadata":{
//         "sys":{
//             "MethodName":"azure",
//             "UtcNow":"2023-09-07T11:40:45.004275Z",
//             "RandGuid":"35e6e68c-5583-436c-a277-5aec2b416ba8"
//         }
//     }
// }
/// https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers#request-payload
#[derive(Debug, Serialize, Deserialize, Default)]
struct AzureInvokeRequest {
    #[serde(rename = "Data")]
    data: serde_json::Value,
    #[serde(rename = "Metadata")]
    metadata: serde_json::Value,
}

/// https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers#response-payload
#[derive(Debug, Serialize, Deserialize, Default)]
struct AzureInvokeResponse {
    #[serde(rename = "Outputs")]
    outputs: Option<serde_json::Value>,
    #[serde(rename = "Logs")]
    logs: Vec<String>,
    #[serde(rename = "ReturnValue")]
    return_value: Option<serde_json::Value>,
}

impl warp::Reply for AzureInvokeResponse {
    fn into_response(self) -> warp::reply::Response {
        Response::builder()
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&self).unwrap().into())
            .unwrap()
    }
}

pub(crate) async fn start_server(tx: Sender<()>) -> Result<()> {
    eprintln!("Starting server for Azure Functions");
    eprintln!("Getting KeyVault secrets");
    let secrets = Arc::new(get_keyvault_secrets(&env::var("KEY_VAULT_NAME")?).await?);
    eprintln!("Creating Splunk client");
    let splunk = Arc::new(Splunk::new(&secrets.splunk_host, &secrets.splunk_token)?);
    splunk
        .log("Starting server / Splunk Client created")
        .await?;
    set_ssphp_run()?;

    let azure_in_progress = Arc::new(Mutex::new(false));

    let azure = warp::post()
        .and(warp::path("azure"))
        .and(warp::body::bytes())
        .then({
            let azure_in_progress = azure_in_progress.clone();
            let azure_splunk = splunk.clone();
            let azure_secrets = secrets.clone();
            move |bytes: Bytes| {
                let in_progress = azure_in_progress.clone();
                let azure_splunk = azure_splunk.clone();
                let azure_secrets = azure_secrets.clone();

                async move {
                    let mut response = AzureInvokeResponse {
                        outputs: None,
                        logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                        return_value: None,
                    };
                    if *in_progress.lock().unwrap() {
                        response.logs.push(
                            "Azure collection is already in progress. NOT starting.".to_owned(),
                        );
                        return response;
                    } else {
                        *in_progress.lock().unwrap() = true;
                        response.logs.push("Aquired lock, starting".to_owned());
                    }
                    let result = match azure(azure_secrets, azure_splunk).await {
                        Ok(_) => "Success".to_owned(),
                        Err(e) => format!("{:?}", e),
                    };
                    response.logs.push(result);
                    *in_progress.lock().unwrap() = false;
                    response
                }
            }
        });

    let m365_in_progress = Arc::new(Mutex::new(false));
    let m365_powershell_installed = Arc::new(Mutex::new(false));
    let m365 = warp::post()
        .and(warp::path("m365"))
        .and(warp::body::bytes())
        .then({
            let m365_in_progress = m365_in_progress.clone();
            let powershell_installed = m365_powershell_installed.clone();
            let m365_splunk = splunk.clone();
            let m365_secrets = secrets.clone();

            move |bytes: Bytes| {
                let in_progress = m365_in_progress.clone();
                let powershell_installed = m365_powershell_installed.clone();
                let m365_splunk = m365_splunk.clone();
                let m365_secrets = m365_secrets.clone();

                async move {
                    let mut response = AzureInvokeResponse {
                        outputs: None,
                        logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                        return_value: None,
                    };
                    if *in_progress.lock().unwrap() {
                        response.logs.push(
                            "Azure collection is already in progress. NOT starting.".to_owned(),
                        );
                        return response;
                    } else {
                        *in_progress.lock().unwrap() = true;
                        response.logs.push("Aquired lock, starting".to_owned());
                    }

                    if !*powershell_installed.lock().unwrap() {
                        m365_splunk.log("Powershell: Installing").await.unwrap();

                        install_powershell().await.unwrap();
                        *powershell_installed.lock().unwrap() = true;

                        m365_splunk
                            .log("Powershell: Install Complete")
                            .await
                            .unwrap();
                    } else {
                        m365_splunk
                            .log("Powershell: already installed")
                            .await
                            .unwrap();
                    }
                    let result = match m365(m365_secrets, m365_splunk).await {
                        Ok(_) => "Success".to_owned(),
                        Err(e) => format!("{:?}", e),
                    };
                    response.logs.push(result);
                    *in_progress.lock().unwrap() = false;
                    response
                }
            }
        });
    let routes = warp::post().and(azure).or(m365);

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };
    let server = tokio::spawn(warp::serve(routes).run((Ipv4Addr::LOCALHOST, port)));
    tx.send(()).unwrap();
    let _ = server.await?;
    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_azure_route() -> Result<()> {
    let (tx, mut rx) = oneshot::channel::<()>();
    tokio::spawn(start_server(tx));
    let _ = rx.await;
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/azure")
        .body("Hello, Azure")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    Ok(())
}
