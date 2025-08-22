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
            return _(['2024-08',
                      '2024-09',
                      '2024-10',
                      '2024-11',
                      '2024-12',
                      '2025-01',
                      '2025-02',
                      '2025-03',
                      '2025-04',
                      '2025-05',
                      '2025-06',
                      '2025-07',
                      '2025-08',
                      '2025-09',
                      '2025-10',
                      '2025-11',
                      '2025-12',
                      '2026-01',
                      '2026-02',
                      '2026-03',
                      '2026-04',
                      '2026-05',
                      '2026-06',
                      '2026-07',
                      '2026-08',
                      '2026-09',
                      '2026-10',
                      '2026-11',
                      '2026-12',
                      '2027-01',
                      '2027-02',
                      '2027-03',
                      '2027-04',
                      '2027-05',
                      '2027-06',
                      '2027-07',
                      '2027-08',
                      '2027-09',
                      '2027-10',
                      '2027-11',
                      '2027-12',
                      '2028-01',
                      '2028-02',
                      '2028-03',
                      '2028-04',
                      '2028-05',
                      '2028-06',
                      '2028-07',
                      '2028-08',
                      '2028-09',
                      '2028-10',
                      '2028-11',
                      '2028-12',
                      '2029-01',
                      '2029-02',
                      '2029-03',
                      '2029-04',
                      '2029-05',
                      '2029-06',
                      '2029-07',
                      '2029-08',
                      '2029-09',
                      '2029-10',
                      '2029-11',
                      '2029-12',
                      '2030-01',
                      '2030-02',
                      '2030-03',
                      '2030-04',
                      '2030-05',
                      '2030-06',
                      '2030-07',
                      '2030-08',
                      '2030-09',
                      '2030-10',
                      '2030-11',
                      '2030-12',
                      'title']).contains(cell.field);
        },
        render: function ($td, cell) {
            var label = cell.value.split("|")[0];
            var val = cell.value.split("|")[1];

            if (val == "green") {
                $td.addClass("range-cell").addClass("css_for_green")
            }
            else if (val == "green2") {
                $td.addClass("range-cell").addClass("css_for_green2")
            }
            else if (val == "orange") {
                $td.addClass("range-cell").addClass("css_for_orange")
            }
            else if (val == "orange2") {
                $td.addClass("range-cell").addClass("css_for_orange2")
            }
            else if (val == "blue") {
                $td.addClass("range-cell").addClass("css_for_blue")
            }
            else if (val == "grey") {
                $td.addClass("range-cell").addClass("css_for_grey")
            }
            else if (val == "red") {
                $td.addClass("range-cell").addClass("css_for_red")
            } else {
                $td.addClass("range-cell").addClass("css_for_white")
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