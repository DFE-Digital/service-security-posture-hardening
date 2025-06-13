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
            return _(['Azure Posture Configuration',
                      'Kubernetes AKS Configuration',
                      'Repository Configuration',
                      'Code Scanning Alerts',
                      'VM Vulnerability Alerts',
                      'POSTURE',
                      'KUBERNETES',
                      'VULNERABILITY',
                      'REPOS',
                      'CODE_SCAN',
                      'POSTURE_abs',
                      'KUBERNETES_abs',
                      'VULNERABILITY_abs',
                      'REPOS_abs',
                      'CODE_SCAN_abs',
                      'POSTURE_perc',
                      'KUBERNETES_perc',
                      'VULNERABILITY_perc',
                      'REPOS_perc',
                      'CODE_SCAN_perc']).contains(cell.field);
        },
        render: function ($td, cell) {
            var label = cell.value.split("|")[0];
            var val = cell.value.split("|")[1];

            if (val == "green") {
                $td.addClass("range-cell").addClass("css_for_green")
            }
            else if (val == "green1") {
                $td.addClass("range-cell").addClass("css_for_green1")
            } 
            else if (val == "green2") {
                $td.addClass("range-cell").addClass("css_for_green2")
            } 
            else if (val == "green3") {
                $td.addClass("range-cell").addClass("css_for_green3")
            } 
            else if (val == "green4") {
                $td.addClass("range-cell").addClass("css_for_green4")
            } 
            else if (val == "green5") {
                $td.addClass("range-cell").addClass("css_for_green5")
            } 
            else if (val == "green6") {
                $td.addClass("range-cell").addClass("css_for_green6")
            } 
            else if (val == "orange") {
                $td.addClass("range-cell").addClass("css_for_orange")
            } 
            else if (val == "orange1") {
                $td.addClass("range-cell").addClass("css_for_orange1")
            } 
            else if (val == "orange2") {
                $td.addClass("range-cell").addClass("css_for_orange2")
            } 
            else if (val == "orange3") {
                $td.addClass("range-cell").addClass("css_for_orange3")
            } 
            else if (val == "orange4") {
                $td.addClass("range-cell").addClass("css_for_orange4")
            } 
            else if (val == "orange5") {
                $td.addClass("range-cell").addClass("css_for_orange5")
            } 
            else if (val == "orange6") {
                $td.addClass("range-cell").addClass("css_for_orange6")
            } 
            else if (val == "red") {
                $td.addClass("range-cell").addClass("css_for_red")
            } 
            else if (val == "red1") {
                $td.addClass("range-cell").addClass("css_for_red1")
            } 
            else if (val == "red2") {
                $td.addClass("range-cell").addClass("css_for_red2")
            } 
            else if (val == "red3") {
                $td.addClass("range-cell").addClass("css_for_red3")
            } 
            else if (val == "red4") {
                $td.addClass("range-cell").addClass("css_for_red4")
            } 
            else if (val == "red5") {
                $td.addClass("range-cell").addClass("css_for_red5")
            } 
            else if (val == "red6") {
                $td.addClass("range-cell").addClass("css_for_red6")
            } 
            else {
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
});