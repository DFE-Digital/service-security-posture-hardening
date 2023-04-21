#!/usr/bin/env python3
import json
import stat
import tarfile
import time
from copy import deepcopy
from datetime import datetime
from pathlib import Path
from pprint import pprint

import click
import requests
from requests.auth import HTTPBasicAuth


class SplunkAppInspectReport:
    def __init__(self, report):
        self.report = report

    def print_failed_checks(self):
        print("\n\n")
        for report in self.report.get("reports", []):
            for group in report.get("groups", []):
                for check in group.get("checks", []):
                    if check.get("result") in ["failure"]:
                        pprint(check, indent=4, width=200)
                        print("\n\n")


class SplunkAppInspect:
    def __init__(self, user, password, token=None, request_id=None, packagetargz=None):
        self.user = user
        self.password = password
        self.headers = {
            "Cache-Control": "no-cache",
        }
        self.token = token
        self.request_id = request_id
        self.packagetargz = packagetargz
        self.timeout = 10

    def make_tarfile(self, source_dir):
        now = int(datetime.now().timestamp())
        path = Path(source_dir)
        if not self.packagetargz:
            self.packagetargz = f"{path.absolute().name}_{now}.tar.gz"
        with tarfile.open(self.packagetargz, "w:gz") as tar:

            def remove_world_writable_permissions(tarinfo):
                tarinfo.mode = tarinfo.mode & ~stat.S_IWOTH
                return tarinfo

            tar.add(
                source_dir,
                arcname=path.absolute().name,
                filter=remove_world_writable_permissions,
            )
        return self.packagetargz

    def login(self):
        url = "https://api.splunk.com/2.0/rest/login/splunk"
        auth = HTTPBasicAuth(self.user, self.password)
        auth_res = requests.get(url, auth=auth, timeout=self.timeout)

        self.token = auth_res.json().get("data", {}).get("token")

        self.headers.update(
            {
                "Authorization": f"bearer {self.token}",
            }
        )

    def submit_package(self):
        validate_url = "https://appinspect.splunk.com/v1/app/validate"

        headers = deepcopy(self.headers)

        headers.update(
            {
                "included_tags": "cloud",
            }
        )

        with open(self.packagetargz, "rb") as targz:
            validate_res = requests.post(
                validate_url,
                files={"app_package": targz},
                headers=headers,
                timeout=self.timeout,
            )

        pprint(validate_res.json(), indent=4, width=200)

        self.request_id = validate_res.json().get("request_id")
        return self

    def wait_for_processing(self):
        status_url = (
            f"https://appinspect.splunk.com/v1/app/validate/status/{self.request_id}"
        )

        sleep = 0
        while True:
            status_res = requests.get(
                status_url, headers=self.headers, timeout=self.timeout
            )
            pprint(status_res.json(), indent=4, width=200)
            time.sleep(sleep)
            sleep += 1
            if status_res.json().get("status", "") != "PROCESSING":
                break

    def get_report(self):
        report_url = f"https://appinspect.splunk.com/v1/app/report/{self.request_id}"

        headers = deepcopy(self.headers)

        headers.update(
            {
                "Content-Type": "application/json",
            }
        )

        report_res = requests.get(report_url, headers=headers, timeout=self.timeout)
        report_json = report_res.json()
        pprint(report_json, indent=4, width=200)
        return report_json

    def validate_package(self, packagetargz):
        self.packagetargz = packagetargz
        self.login()
        self.submit_package()
        self.wait_for_processing()
        report = self.get_report()
        return report

    def package_then_validate(self, app_directory):
        self.make_tarfile(app_directory)
        self.login()
        self.submit_package()
        self.wait_for_processing()
        report = self.get_report()
        with open(f"{self.packagetargz}_report.json", "w") as f:
            json.dump(report, f)
        return report


@click.command()
@click.argument(
    "app_package",
    type=click.Path(exists=True, readable=True),
)
@click.option(
    "--splunkuser",
    envvar="SPLUNK_USER",
    help="The splunk.com username. Can also be set via SPLUNK_USER environment variable",
    type=str,
    required=True,
)
@click.option(
    "--splunkpassword",
    envvar="SPLUNK_PASSWORD",
    help="The splunk.com password. Can also be set via SPLUNK_USER environment variable",
    type=str,
    required=True,
)
@click.option(
    "--justvalidate",
    help="Provied a package .tag.gz instead of a directory and validate it.",
    type=bool,
    required=False,
    default=False,
    is_flag=True,
)

@click.option(
    "--outfile",
    help="Provied a package .tag.gz instead of a directory and validate it.",
    type=str,
    required=False,
    default=None,
)

def main(app_package, splunkuser, splunkpassword, justvalidate, outfile):
    sai = SplunkAppInspect(splunkuser, splunkpassword, packagetargz=outfile)
    if justvalidate:
        report = sai.validate_package(app_package)
    else:
        report = sai.package_then_validate(app_package)
    report = SplunkAppInspectReport(report)
    report.print_failed_checks()


if __name__ == "__main__":
    main()
