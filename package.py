#!/usr/bin/env python3
import configparser
import json
import shutil
import stat
import tarfile
import time
import glob
from copy import deepcopy
from datetime import datetime
from distutils.dir_util import copy_tree
from pathlib import Path
from pprint import pprint

import click
import requests
from requests.auth import HTTPBasicAuth
import confreader
import sys


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

    def increment_build_numbers(self, app_directory):
        config = configparser.ConfigParser()
        config.read(f"{app_directory}/default/app.conf")

        version = config["launcher"]["version"]

        sem_ver = version.split(".")
        sem_ver[-1] = str(int(sem_ver[-1]) + 1)
        new_version = ".".join(sem_ver)

        config["launcher"]["version"] = new_version
        config["id"]["version"] = new_version
        config["install"]["build"] = str(time.time()).split(".")[0]

        with open(f"{app_directory}/default/app.conf", "w") as f:
            config.write(f)


    def copy_app(self, app_directory, suffix=""):
        target_dir = f"target/{app_directory[:-1]}{suffix}"
        try:
            shutil.rmtree(target_dir)
        except:
            pass

        copy_tree(app_directory, target_dir)

        return target_dir



    def replace_dev_tag(self, app_directory, environment=""):
        file_list = glob.glob(f"{app_directory}/**/*.conf",recursive=True)
        file_list = file_list + glob.glob(f"{app_directory}/**/*.json",recursive=True)
        file_list = file_list + glob.glob(f"{app_directory}/**/*.xml",recursive=True)

        for each in file_list:
            with open(each, 'r') as f:
                s = f.read()

            with open(each, 'w') as f:
                s = s.replace("~^ENV^~", environment)
                f.write(s)


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
    "--dev",
    help="Build a DEV package",
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
def main(app_package, splunkuser, splunkpassword, justvalidate, outfile, dev):
    sai = SplunkAppInspect(splunkuser, splunkpassword, packagetargz=outfile)
    sai.increment_build_numbers(app_package)

    if dev:
        app_package = sai.copy_app(app_package, suffix="_DEV")
        sai.replace_dev_tag(app_package, "_DEV")
        conf_reader_result = confreader.result(app_package)
        if conf_reader_result:
            report = sai.package_then_validate(app_package)
        else:
            sys.exit(1)

    else:
        if justvalidate:
            report = sai.validate_package(app_package)
        else:
            app_package = sai.copy_app(app_package)
            sai.replace_dev_tag(app_package)
            conf_reader_result = confreader.result(app_package)
            if conf_reader_result:
                report = sai.package_then_validate(app_package)
            else:
                sys.exit(1)

    report = SplunkAppInspectReport(report)
    report.print_failed_checks()


if __name__ == "__main__":
    main()
