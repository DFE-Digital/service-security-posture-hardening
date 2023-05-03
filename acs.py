import json
import requests
import os
from pprint import pprint


class SplunkACS:
    def __init__(self, stack, token):
        self.stack = stack
        self.token = token
        self.url = f"https://admin.splunk.com/{self.stack}/adminconfig/v2/apps?count=100"
        self.headers = {"Authorization": f"Bearer {self.token}"}
        self.ack_header = {"ACS-Legal-Ack": "Y"}
        self.all_headers = dict(self.ack_header, **self.headers)

    def get_app_list(self):
        r = requests.get(self.url, headers=self.headers)

        return r.json()["apps"]

    def check_app_exists(self, app):
        self.app = app
        app_list = self.get_app_list()
        
        for app in app_list:
            if app["name"] == self.app:
                return True
            
        return False

    def install_app(self, app_path):
        print(f"gonna install {app_path}")
        print(self.all_headers)
        
        files = {"package": open(app_path,"rb"), "token": "token"}
        response = requests.post(self.url, headers=self.all_headers, files = files)

        return response


def main():
    token = os.getenv('ACS_TOKEN')
    acs = SplunkACS("dfe", token)
    # app = "Splunk_TA_MS_Security"
    app_path="SSPHP_DEV_1683125797.tar.gz"

    # acs_response = acs.check_app_exists(app)
    # print(f"the app is {app} and exists is {acs_response}")

    # if acs_response:
    success = acs.install_app(app_path)
    print(success.text)

if __name__ == "__main__":
    main()
