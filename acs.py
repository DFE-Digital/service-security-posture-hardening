import json
import requests
import os
from pprint import pprint


class SplunkACS:
    def __init__(self, stack, acs_token, validation_token):
        self.stack = stack
        self.acs_token = acs_token
        self.validation_token = validation_token
        self.url = f"https://admin.splunk.com/{self.stack}/adminconfig/v2/apps/victoria"
        self.headers = {"Authorization": f"Bearer {self.acs_token}", "X-Splunk-Authorization": self.validation_token}
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
        print(f"Starting install of {app_path}")
        
        with open(app_path,"rb") as f:
            data = f.read()

        response = requests.post(self.url, headers=self.all_headers, data = data)

        return response
