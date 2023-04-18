#!/bin/bash
set -xe

# pip install -r tests/requirements.txt splunk-add-on-ucc-framework splunk-packaging-toolkit

ruff TA_SSPHP/bin/*.py tests/*.py --ignore=F401,E501,E402

cd tests; pytest; cd ..

cd tests; pytest -m live; cd ..

pip-audit -S

bandit TA_SSPHP/bin/*.py

pycodestyle TA_SSPHP/bin/*.py --ignore=E501,W503,W504

pylint --fail-under 0 TA_SSPHP/bin/*.py tests/*.py

ucc-gen build --source TA_SSPHP && slim package output/TA_SSPHP && ls -la *.gz && sha1sum *.gz

