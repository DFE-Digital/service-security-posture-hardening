require([
    'underscore',
    'jquery',
    'splunkjs/mvc',
    'splunkjs/mvc/tableview',
    'splunkjs/mvc/simplexml/ready!'
], function (_, $, mvc, TableView) {

    var CustomRangeRenderer = TableView.BaseCellRenderer.extend({
        canRender: function (cell) {
            return _(['BizTalk', 'WebShop']).contains(cell.field);
        },
        render: function ($td, cell) {
            var label = cell.value.split("|")[0];
            var val = cell.value.split("|")[1];

            if (val == "green") {
                $td.addClass("range-cell").addClass("css_for_green")
            }
            else if (val == "yellow") {
                $td.addClass("range-cell").addClass("css_for_yellow")
            }
            else if (val == "red") {
                $td.addClass("range-cell").addClass("css_for_red")
            } else {
                $td.addClass("range-cell").addClass("css_for_grey")
            }
            $td.text(label).addClass("string");
        }
    });

    var sh = mvc.Components.get("tableWithColorBasedOnAnotherField");
    if (typeof (sh) != "undefined") {
        sh.getVisualization(function (tableView) {
            // Add custom cell renderer and force re-render
            tableView.table.addCellRenderer(new CustomRangeRenderer());
            tableView.table.render();
        });
    }
});