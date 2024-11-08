use serde::Deserialize;
use serde::Serialize;
use valuable::Valuable;

// /// Example data for Azure Functions request/response structs
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
#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub struct AzureInvokeRequest {
    #[serde(rename = "Data")]
    pub(crate) data: Data,
    #[serde(rename = "Metadata")]
    pub(crate) metadata: Metadata,
}

#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub(crate) struct Data {
    pub(crate) timer: Option<Timer>,
}

#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub struct Timer {
    #[serde(rename = "IsPastDue")]
    pub(crate) is_past_due: bool,
    #[serde(rename = "Schedule")]
    pub(crate) schedule: Schedule,
    #[serde(rename = "ScheduleStats")]
    pub(crate) schedule_stats: Option<()>,
}

#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub struct Schedule {
    #[serde(rename = "AdjustForDst")]
    pub(crate) adjust_for_dst: bool,
}

#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub struct Metadata {
    #[allow(dead_code)]
    pub(crate) sys: MetadataSys,
}

#[derive(Debug, Deserialize, Default, Serialize, Valuable)]
pub struct MetadataSys {
    #[serde(rename = "MethodName")]
    pub(crate) method_name: String,
    #[serde(rename = "RandGuid")]
    pub(crate) rand_guid: String,
    #[serde(rename = "UtcNow")]
    pub(crate) utc_now: String,
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
