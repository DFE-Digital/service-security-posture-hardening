# Code Management

There are 2 apps in the SplunkCloud system - **SSPHP** is the Production app which Users have access to, and **SSPHP_DEV** is the development app where everything is built and tested. BUT, there is only a single code base in Github from which both of these apps are built, validated, and deployed to SplunkCloud by the automation.

## Code Management in Splunk

### Philosophy
No code in the Production App has been created or managed using the Splunk Web User Interface (WebUI). All code is built and tested using the WebUI in the _DEV app, and then transposed into .conf files in Github. Once the changes have been replicated in the Github files, then all the WebUI versions of the files are deleted and the clean .conf version of the app is again deployed to the _DEV app. Only once you are satisfied that the app is ready to release - working, correct, tested, and with a user base ready to receive it - can it be deployed to the Production app.

All SSPHP code in the Production app is created and managed in .conf files, and only in .conf files. *please never edit any Splunk Knowledge Objects in the Production app using the Splunk WebUI because this will create conflicts of precedence that you will find it difficult to resolve*


### Github Repos
The Repo is held in the Github Organisation called (DFE-Digital)[https://github.com/DFE-Digital/service-security-posture-hardening]. You will need to request access to the Repo from TBD.

The Repo is organised with a directory for (SSPHP)[./SSPHP] (the app that is deployed onto the Splunk Search Head), and a directory for the (TA)[./TA_SSPHP] code (that is deployed to the (Heavy Forwarder)[./TA Heavy Forwarder Architecture.md]).

---
## Deployment Process

### 3 Steps : Packaging, Validation & Deployment

1. **Packaging** : We take a copy of the directory SSPHP, we modify the files to indicate the environment it will be deployed to and to reformat .conf files for SplunkCloud, and we create a tarball and compress it with gzip
2. **Validation** : We submit the app to the Splunk app validation endpoint [https://dev.splunk.com/enterprise/docs/releaseapps/cloudvetting/]  
3. **Deployment** : Using the Splunk ACS [https://docs.splunk.com/Documentation/SplunkCloud/9.0.2303/Config/ACSIntro] we deploy the app to the AdHoc searchhead. 

All of these steps are performed by (package.py)[../package.py]

To use package.py you will need valid credentials for SplunkBase and a deployment token from the AdHoc searchhead. These should be set as Environment Variables as follows :
SPLUNK_USER = "********"
SPLUNK_PASSWORD = "*********"
ACS_TOKEN = "************"

To execute pckage.py use `.\package.py .\SSPHP\SSPHP\ --dev`. The --dev option will deploy to the _DEV version of the app, so leave that off to deploy to Production.


### How source files are modified by the Deploymnent Process
- Splunk insists that all searches held in macros.conf and savedsearches.conf have each line terminated with a \ character; despite this not being the case when the search is run through the WebUI. To avoid this time-consuming and error-prone part of creating these knowledge objects, the deployment packager will automatically add these characters when it packages the files. In order for this to work, each code block needs to be encased, before and after, by """ (three consecutive double-quote marks).
- app.conf has a number of modifications made to it as part of the deployment. The app version number is bumped by 1 (minor) increment, and the build is replaced by the current epoch time. Also, package id and label values have an environment variable appended to them in the repo editable version ~^ENV^~. This will be replace by _DEV if the build for a dev version of the app (SSPHP_DEV) or be left null for a production build.
- App Name and associated file names and folders will be modified to SSPHP_DEV as above in the built files that are sent to Splunk, depending on whether the build is for dev or prod apps.


---
## App Config - things to note
```INI
[shclustering]
deployer_lookups_push_mode = preserve_lookups
```

This setting in app.conf must be as above because otherwise the lookup files in the app will be overwritten every time a new version of the app is deployed.