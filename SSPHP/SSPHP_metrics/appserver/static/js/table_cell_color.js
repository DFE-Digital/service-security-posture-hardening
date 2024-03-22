require([
    'underscore',
    'jquery',
    'splunkjs/mvc',
    'splunkjs/mvc/tableview',
    'splunkjs/mvc/simplexml/ready!'
], function (_, $, mvc, TableView) {

    var CustomRangeRenderer = TableView.BaseCellRenderer.extend({
        canRender: function (cell) {
            //return true;
            return _(['Score',
                      'security_advisory.severity',
                      'created_at_age',
                      'created_at',
                      'Compliance Status',
                      'properties.resourceDetails.ResourceName',
                      'properties.status.code','AAD-01','AAD-02',
                      'AAD-03','AAD-04','AAD-05','AZURE-01 : Identity and Access Management',
                      'AZURE-02 : Microsoft Defender','AZURE-03 : Storage Accounts',
                      'AZURE-04 : Database Services','AZURE-05 : Logging and Monitoring',
                      'AZURE-06 : Networking','AZURE-07 : Virtual Machines','AZURE-08 : Key Vault','AZURE-09 : AppService',
                      'AZURE-10 : Miscellaneous',
                      'DNS-01',
                      'DNS-02',
                      'DNS-03',
                      'DNS-04',
                      'DNS-05']).contains(cell.field);
        },
        render: function ($td, cell) {
            var label = cell.value.split("|")[0];
            var val = cell.value.split("|")[1];

            if (val == "green") {
                $td.addClass("range-cell").addClass("css_for_green")
            }
            else if (val == "orange") {
                $td.addClass("range-cell").addClass("css_for_orange")
            }
            else if (val == "red") {
                $td.addClass("range-cell").addClass("css_for_red")
            } else {
                $td.addClass("range-cell").addClass("css_for_blue")
            }
            $td.text(label).addClass("string");
        }
    });

    var sh1 = mvc.Components.get("table1");
    if (typeof (sh1) != "undefined") {
        sh1.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }

    var sh2 = mvc.Components.get("table2");
    if (typeof (sh2) != "undefined") {
        sh2.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }
});