use serde::Deserialize;
use serde::Serialize;

/// Example data for Azure Functions request/response structs
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
#[derive(Debug, Deserialize, Default, Serialize)]
pub(crate) struct AzureInvokeRequest {
    #[allow(dead_code)]
    #[serde(rename = "Data")]
    pub(crate) data: serde_json::Value,
    #[allow(dead_code)]
    #[serde(rename = "Metadata")]
    pub(crate) metadata: serde_json::Value,
}

/// https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers#response-payload
#[derive(Debug, Serialize, Default)]
pub(crate) struct AzureInvokeResponse {
    #[serde(rename = "Outputs")]
    pub(crate) outputs: Option<serde_json::Value>,
    #[serde(rename = "Logs")]
    pub(crate) logs: Vec<String>,
    #[serde(rename = "ReturnValue")]
    pub(crate) return_value: Option<serde_json::Value>,
}
