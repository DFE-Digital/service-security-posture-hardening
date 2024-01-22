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
            return _(['state','conditions.locations.includeLocations{}','conditions.locations.excludeLocations{}','grantControls.builtInControls{}','conditions.users.includeGroups{}','conditions.clientAppTypes{}','grantControls.builtInControls{}','isEnabled','implementationStatus','isTrusted','ipRanges{}.cidrAddress']).contains(cell.field);
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

    var sh1 = mvc.Components.get("table3");
    if (typeof (sh1) != "undefined") {
        sh1.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }

    var sh2 = mvc.Components.get("table4");
    if (typeof (sh2) != "undefined") {
        sh2.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }
});