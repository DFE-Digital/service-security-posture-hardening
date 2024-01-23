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
            return _(['state',
                      'defaultUserRolePermissions.allowedToCreateTenants',
                      'conditions.locations.includeLocations{}',
                      'conditions.locations.excludeLocations{}',
                      'conditions.users.includeUsers{}',
                      'conditions.users.includeGroups{}',
                      'conditions.clientAppTypes{}',
                      'grantControls.builtInControls{}',
                      'conditions.signInRiskLevels{}',
                      'isEnabled',
                      'implementationStatus',
                      'isTrusted',
                      'values{}.value',
                      'values{}.name',
                      'permissionGrantPolicyIdsAssignedToDefaultUserRole{}',
                      'defaultUserRolePermissions.allowedToCreateApps',
                      'guestUserRoleId',
                      'allowInvitesFrom',
                      'defaultUserRolePermissions.allowedToCreateSecurityGroups',
                      'properties.type',
                      'properties.enabled',
                      'properties.assignableScopes{}',
                      'properties.permissions{}.actions{}',
                      'properties.enforcementMode',
                      'properties.autoProvision',
                      'properties.alertNotifications.state',
                      'properties.notificationsByRole.state',
                      'properties.notificationsByRole.roles{}',
                      'properties.emails',
                      'properties.alertNotifications.minimalSeverity',
                      'permissions{}.actions{}',
                      'roleName',
                      'blockSubscriptionsIntoTenant',
                      'blockSubscriptionsLeavingTenant',
                      'kind',
                      'properties.serverKeyType',
                      'uri',
                      'ipRanges{}.cidrAddress',
                      'alternateContactType',
                      'emailAddress',
                      'name',
                      'phoneNumber',
                      'SummaryMap.AccountAccessKeysPresent',
                      'SummaryMap.AccountMFAEnabled',
                      'serialNumber',
                      'SummaryMap.AccountMFAEnabled',
                      'number_with_AccountMFAEnabled',
                      'MinimumPasswordLength',
                      'PasswordReusePrevention',
                      'password_enabled',
                      'mfa_active',
                      'access_key_1_last_used_date',
                      'title']).contains(cell.field);
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