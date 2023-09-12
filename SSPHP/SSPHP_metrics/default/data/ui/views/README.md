# Dashboards
There are 3 dashboards in the app : 

ssphp_metrics_dashboard.xml
---------------------------
This is the main user dashoard. The xml source is split into 3 parts which are contained in the folder ssphp_metrics_dashboard.d, and none of these is a complete dashboard file on its own. In order to put these together into a single functioning dasboard run &'C:\Program Files\Python310\python.exe' .\build_metrics_dashboard.py. The resaon for having the parts broken out this way is (a) because it is a 3000 line xml file so navigating it is a pain, and (b) the middle bit is exactly the same for every row on the dashboard, with just the data source changing. So rather than having to make the exact same changes multiple times for each row, the build script makes the changes and concatenates the row for you.


ssphp_metrics_dashboard_drilldown.xml
-------------------------------------
This is the drilldown dashboard based on clicking panels in the above main dashboard. The main dashboard passes the use case context to the drilldown which gets the data using the same macro as it's equivalent use case savedsearch.


ssphp_metrics_dashboard_controls.xml
------------------------------------
This is a list of sshp metrics controls, and the current metric value for the use cases for each control.