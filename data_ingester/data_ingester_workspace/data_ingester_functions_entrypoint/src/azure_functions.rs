use data_ingester_ms_powershell::runner::powershell;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;
use warp::{http::Response, Filter};

use data_ingester_aws::aws::aws;
use data_ingester_supporting::keyvault::get_keyvault_secrets;
//use data_ingester_ms_graph::ms_graph::azure_users;
use anyhow::Result;
use data_ingester_azure::azure_users;
use data_ingester_ms_graph::ms_graph::m365;
use data_ingester_ms_powershell::powershell::install_powershell;
use data_ingester_splunk::splunk::set_ssphp_run;
use data_ingester_splunk::splunk::Splunk;
use memory_stats::memory_stats;

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
        let response = Response::builder().header("Content-Type", "application/json");

        let response = match serde_json::to_string(&self) {
            Ok(json) => response.body(json.into()),
            Err(err) => response
                .status(500)
                .body(format!(r#"{{"error": "{err:?}"}}"#).into()),
        };

        match response {
            Ok(response) => response,
            Err(err) => Response::builder()
                .status(500)
                .body(err.to_string().into())
                .expect("Minimal error response should build correctly"),
        }
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

    let azure_in_progress = Arc::new(Mutex::new(()));

    let azure = warp::post().and(warp::path("azure")).then({
        let azure_in_progress = azure_in_progress.clone();
        let azure_splunk = splunk.clone();
        let azure_secrets = secrets.clone();
        move || {
            let in_progress = azure_in_progress.clone();
            let azure_splunk = azure_splunk.clone();
            let azure_secrets = azure_secrets.clone();

            async move {
                let mut response = AzureInvokeResponse {
                    outputs: None,
                    logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                    return_value: None,
                };

                let lock = match in_progress.try_lock() {
                    Ok(lock) => {
                        response.logs.push("Aquired lock, starting".to_owned());
                        lock
                    }
                    Err(_) => {
                        response.logs.push(
                            "Azure collection is already in progress. NOT starting.".to_owned(),
                        );
                        return response;
                    }
                };

                let result = match azure_users(azure_secrets, azure_splunk).await {
                    Ok(_) => "Success".to_owned(),
                    Err(e) => format!("{:?}", e),
                };
                response.logs.push(result);
                if let Some(usage) = memory_stats() {
                    println!(
                        "Current physical memory usage: {}",
                        usage.physical_mem / 1_000_000
                    );
                    println!(
                        "Current virtual memory usage: {}",
                        usage.virtual_mem / 1_000_000
                    );
                } else {
                    println!("Couldn't get the current memory usage :(");
                }
                drop(lock);
                response
            }
        }
    });

    let m365_in_progress = Arc::new(Mutex::new(()));
    let m365 = warp::post()
        .and(warp::path("m365"))
        //        .and(warp::body::bytes())
        .then({
            let m365_in_progress = m365_in_progress.clone();
            let m365_splunk = splunk.clone();
            let m365_secrets = secrets.clone();

            move || {
                let in_progress = m365_in_progress.clone();
                let m365_splunk = m365_splunk.clone();
                let m365_secrets = m365_secrets.clone();

                async move {
                    eprintln!("GIT_HASH: {}", env!("GIT_HASH"));

                    let mut response = AzureInvokeResponse {
                        outputs: None,
                        logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                        return_value: None,
                    };
                    let lock = match in_progress.try_lock() {
                        Ok(lock) => {
                            m365_splunk
                                .log("Aquired lock, starting")
                                .await
                                .expect("Splunk should be available for logging");
                            lock
                        }
                        Err(_) => {
                            m365_splunk
                                .log("M365 collection is already in progress. NOT starting.")
                                .await
                                .expect("Splunk should be available for logging");
                            response.logs.push(
                                "M365 collection is already in progress. NOT starting.".to_owned(),
                            );
                            return response;
                        }
                    };

                    let result = match m365(m365_secrets, m365_splunk).await {
                        Ok(_) => "Success".to_owned(),
                        Err(e) => format!("{:?}", e),
                    };
                    response.logs.push(result);
                    if let Some(usage) = memory_stats() {
                        println!(
                            "Current physical memory usage: {}",
                            usage.physical_mem / 1_000_000
                        );
                        println!(
                            "Current virtual memory usage: {}",
                            usage.virtual_mem / 1_000_000
                        );
                    } else {
                        println!("Couldn't get the current memory usage :(");
                    }
                    drop(lock);
                    response
                }
            }
        });

    let powershell_in_progress = Arc::new(Mutex::new(()));
    let powershell_installed = Arc::new(Mutex::new(false));
    let powershell = warp::post().and(warp::path("powershell")).then({
        let powershell_in_progress = powershell_in_progress.clone();
        let powershell_installed = powershell_installed.clone();
        let powershell_splunk = splunk.clone();
        let powershell_secrets = secrets.clone();

        move || {
            let in_progress = powershell_in_progress.clone();
            let powershell_installed = powershell_installed.clone();
            let powershell_splunk = powershell_splunk.clone();
            let powershell_secrets = powershell_secrets.clone();

            async move {
                eprintln!("GIT_HASH: {}", env!("GIT_HASH"));

                let mut response = AzureInvokeResponse {
                    outputs: None,
                    logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                    return_value: None,
                };
                let lock = match in_progress.try_lock() {
                    Ok(lock) => {
                        powershell_splunk
                            .log("Aquired lock, starting")
                            .await
                            .expect("Splunk should be available for logging");
                        lock
                    }
                    Err(_) => {
                        powershell_splunk
                            .log("Powershell collection is already in progress. NOT starting.")
                            .await
                            .expect("Splunk should be available for logging");
                        response.logs.push(
                            "Powershell collection is already in progress. NOT starting."
                                .to_owned(),
                        );
                        return response;
                    }
                };

                if !*powershell_installed.lock().await {
                    powershell_splunk
                        .log("Powershell: Installing")
                        .await
                        .expect("Splunk should be available for logging");

                    install_powershell()
                        .await
                        .expect("Powershell should install cleanly in the Azure Function instance");
                    *powershell_installed.lock().await = true;
                } else {
                    powershell_splunk
                        .log("Powershell: already installed")
                        .await
                        .expect("Splunk should be available for logging");
                }
                let result = match powershell(powershell_secrets, powershell_splunk).await {
                    Ok(_) => "Success".to_owned(),
                    Err(e) => format!("{:?}", e),
                };
                response.logs.push(result);
                if let Some(usage) = memory_stats() {
                    println!(
                        "Current physical memory usage: {}",
                        usage.physical_mem / 1_000_000
                    );
                    println!(
                        "Current virtual memory usage: {}",
                        usage.virtual_mem / 1_000_000
                    );
                } else {
                    println!("Couldn't get the current memory usage :(");
                }
                drop(lock);
                response
            }
        }
    });

    let aws_in_progress = Arc::new(Mutex::new(()));
    let aws = warp::post().and(warp::path("aws")).then({
        let aws_in_progress = aws_in_progress.clone();
        let aws_splunk = splunk.clone();
        let aws_secrets = secrets.clone();

        move || {
            let in_progress = aws_in_progress.clone();
            let aws_splunk = aws_splunk.clone();
            let aws_secrets = aws_secrets.clone();

            async move {
                eprintln!("GIT_HASH: {}", env!("GIT_HASH"));

                let mut response = AzureInvokeResponse {
                    outputs: None,
                    logs: vec![format!("GIT_HASH: {}", env!("GIT_HASH"))],
                    return_value: None,
                };
                let lock = match in_progress.try_lock() {
                    Ok(lock) => {
                        aws_splunk
                            .log("Aquired lock, starting")
                            .await
                            .expect("Splunk should be available for logging");
                        lock
                    }
                    Err(_) => {
                        aws_splunk
                            .log("AWS collection is already in progress. NOT starting.")
                            .await
                            .expect("Splunk should be available for logging");
                        response.logs.push(
                            "AWS collection is already in progress. NOT starting.".to_owned(),
                        );
                        return response;
                    }
                };

                let result = match aws(aws_secrets, aws_splunk).await {
                    Ok(_) => "Success".to_owned(),
                    Err(e) => format!("{:?}", e),
                };

                response.logs.push(result);
                if let Some(usage) = memory_stats() {
                    println!(
                        "Current physical memory usage: {}",
                        usage.physical_mem / 1_000_000
                    );
                    println!(
                        "Current virtual memory usage: {}",
                        usage.virtual_mem / 1_000_000
                    );
                } else {
                    println!("Couldn't get the current memory usage :(");
                }
                drop(lock);
                response
            }
        }
    });

    let routes = warp::post().and(azure).or(m365).or(powershell).or(aws);

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };
    let server = tokio::spawn(warp::serve(routes).run((Ipv4Addr::LOCALHOST, port)));
    tx.send(())
        .expect("Caller should be listening for Warp start event");
    server.await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use tokio::sync::oneshot::channel;

    use crate::azure_functions::start_server;
    #[ignore]
    #[tokio::test]
    async fn test_azure_route() -> Result<()> {
        let (tx, rx) = channel::<()>();
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
}
