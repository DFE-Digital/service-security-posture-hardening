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
        
        files = {"package": open(app_path,"rb"), "token": "eyJraWQiOiJCSG1zbWdSdDFRVjNjRE1aLWhOTWZmdkJIQ3FUbDNhTmZYMWxqa2U5QnRBIiwiYWxnIjoiUlMyNTYifQ.eyJ2ZXIiOjEsImp0aSI6IkFULmNQVE9QcTF4bml0T2ZrTE95TFQtN3F5TXVyU21zNldrYVByY2JtR3llZFkiLCJpc3MiOiJodHRwczovL2lkcC5sb2dpbi5zcGx1bmsuY29tL29hdXRoMi9hdXNnemp3c2V0Y2hQVklsZzJwNyIsImF1ZCI6ImFwaS5zcGx1bmsuY29tIiwiaWF0IjoxNjgzMTI1Nzk5LCJleHAiOjE2ODMxNTQ1OTksImNpZCI6IjBvYWd6anZwcXRzZHRZUDM1MnA3IiwidWlkIjoiMDB1ZXA4aHlncEpaZ2FMUEwycDciLCJzY3AiOlsib3BlbmlkIl0sImF1dGhfdGltZSI6MTY4MzEyNTc5OSwic3ViIjoiaWFuX3BlYXJsIiwidW5hbWUiOiJpYW5fcGVhcmwiLCJuYW1lIjoiSWFuIFBlYXJsIiwidXNlciI6eyJlbWFpbCI6ImlhbkBwM2FybC5jb20ifSwiZW1haWwiOiJpYW5AcDNhcmwuY29tIn0.JfeS7DP9EDnkztZp0Is5-aCfBHDIM3xBWpM447DtR0LIr3OuPjxdzkRtb3nHq-M23kzG6XAc536z9nEBdzuCpRTpQr-qNbNT3RBm57-Hy8u9q8CY_bc0cZNXax-_6QZrmBJW7WbltXcKQHKy2TMZnOLY7MR6z95CTCWTO8wb7M-akri_geeDyklf1OeSBgPlSZYDfqYiISoTaAfyMzMVK0wCWQMmTPmbAqUX2Jw2PWh6mpehAuWbIev1iMvw0ldRnn2i5RS69SYR-eQ292e-pMwoLaGHfGKLjpZMWWhDAU3-4lupsDimEGX1zzPxtnTy8fK-dfQJNQrO39xfzTzRug"}
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
