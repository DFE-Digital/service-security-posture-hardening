# Data Ingester

This is the application code to collect data from various APIs, transform it into Splunk events, and deliver it a Splunk HEC.

The data collected from each API is required to satisify [CIS Security Benchmarks](https://www.cisecurity.org/cis-benchmarks) 

Data Ingester is composed of multiple crates, each with a clearly defined use.

## Sub-crates (check name of this, could be packages)

### [data_ingester_aws](data_ingester_aws/README.md)
Collect data from AWS

### [data_ingester_axum_entrypoint](data_ingester_axum_entrypoint/README.md)
When running in Azure this is the Azure Functions custom handler

### [data_ingester_azure](data_ingester_azure/README.md)
What is the difference between Azure and Azure rest?

### [data_ingester_azure_rest](data_ingester_azure_rest/README.md)
What is the difference between Azure and Azure rest?

### [data_ingester_github](data_ingester_github/README.md)
Collect data from GitHub

### [data_ingester_ms_graph](data_ingester_ms_graph/README.md)
Collect data from MS Graph

### [data_ingester_ms_powershell](data_ingester_powershell/README.md)
Used for MS data gathering that is only available via powershell modules

### [data_ingester_qualys](data_ingester_qualys/README.md)
Use Qualys API to get CVE's from QID(maybe other way round. need to check)

### [data_ingester_sarif](data_ingester_sarif/README.md)
Crate for working with Sarif files CodeQL/SemGrep

### [data_ingester_splunk](data_ingester_splunk/README.md)
Crate to communicate with Splunk.

Provides a client for sending HEC formated events and traits to make
it easy to convert any implementing struct into a collection of HEC
events.

### [data_ingester_splunk_search](data_ingester_splunk_search/README.md)
Simple client to perform searches against a Splunk instance and get the results back into a struct.

### [data_ingester_supporting](data_ingester_supporting/README.md)
Miscalanious tools used across multiple crates

- DNS - make DNS requests
- Azure KeyVault - Collect Secrets and 

### [data_ingester_threagile](data_ingester_threagile/README.md)

Run [threagile](https://github.com/Threagile/threagile) on a collection of resources and send the results to Splunk
