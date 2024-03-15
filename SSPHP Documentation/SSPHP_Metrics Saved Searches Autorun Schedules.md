**SSPHP_Metrics Saved Searches Autorun Schedules**

Each Use Case is driven by a single Splunk SaveSearch. The data comes from Azure Functions which run daily at between 2am and 3:30am, depending on which API/SDK is being used to source the data). These run every 30 minutes and take approximately 10 minutes each to complete. So the assumption is that they will all have completed before 4am  day.

The Use Case Saved Searches are scheduled - using a cron - to each run once every day. Given that theere are a reasonably large number of Use Case savedsearches, it seems sensible to stagger running them over time. The chosen period is after the Azure Functions will have completed and before users are likely to be working ie 4am to 7am. The schedules are as follows : 

**Hours**
3:30am = Defender Assessments
4am = M365
5am = DNS (AWS)
6am = Azure
7am = Entra AAD

**Minutes**
001 = 10 past the hour
002 = 30
003 = 35
004 = 40
005 = 45
006 = 50
dfe = 55

So the Saved Search 'ssphp_use_case_m365_004_cis_4-6' is scheduled to run at 04:40am each day.


The SavedSearches that combine together the Use Case Outputs and pre-process them for the dashboards (to improve dashboard load times) are scheduled to run every 15 minutes. This is because the summaries created are consumed by the dashboards as 'loadjobs', and are consequently reliant on tplunk artefacts from the savedsearch continuing to persist. In case there has been a Splunk restart or error during the day that has caused those artefacts to have died, it seems sensible to be re-creating them more frequently even though the underlying data will not have changed since the prior morning run. 



