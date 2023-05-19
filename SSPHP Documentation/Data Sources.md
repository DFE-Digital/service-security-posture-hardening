Data Sources

What comes from each....

Azure Findings
Azure Alerts
Github
Qualys


Refer to <ins>here</ins> to see how the data gets into Splunk from each of these sources



**SSPHP_RUN**
Throughout the app, 



**Field Naming**
By convention, all fields that are created in the app and persisted will be prefixed with ssphp_ in order to differentiate them from fields that are from an original data source.



**Normalisation of Source Data**
Some of the data, for example from Azure Defender for Cloud, comes in ways that would be difficult to transform inside Splunk - multiple tables to be joined together which would break limits and be very slow. Also, where different API calls are required to get details of Subscriptions or Resource Groups from the calls to get the Findings or Alerts data; so the TAs have been written so that these transformation and joins have been 'pre-cooked' and joined together in the TA such that the querying of the data in Splunk is straightforward and quick.

The source data from the TA will always contain everything from the original technology. In Splunk, Field Calcs in props.conf have been used to extract the fields needs and assign ssphp names in a relatively standard way.

Where it simplifies things, automatic lookups have been assigned to the sourcetypes to enrich the source data with details of the service to which it relates
see page relating to lookup files <ins>here</ins>

**Indexes & Sourcetypes that Original Data Goes to**
