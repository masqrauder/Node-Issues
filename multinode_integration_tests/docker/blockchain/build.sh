#!/bin/bash -evx
# Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.

[[ $GITHUB_ACTIONS -eq true ]] && sudo --preserve-env ../../../node/ci/free-port-53.sh
docker build -t ganache-cli .
