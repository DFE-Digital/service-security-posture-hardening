Consolidated Findings Dashboard

**Data Sources**
All of the data panels in the dashboard are driven from the data model 
Accelerated

**Role Based Access Control**
Who can see what - how it looks different for a 'Service' user to how it looks for an admin user
see Splunk Access and Permissioning for how users and roles are to be configured
How this works in the xml


**Standard Fields**



**Table Presentation**
The dashboard presents data in Splunk Tables by invoking HTML tags using bespoke JavaScipt code. In order to make the large amount 
Special Javascript... addtags.js is where to find that
How it works - look in macro `ssphp_add_display_colours{{environment}}` for execution
¬¬~!span
~!\/span~!"¬¬


**Drilldown Dashboard**
Sibgle dashboard that gets 
