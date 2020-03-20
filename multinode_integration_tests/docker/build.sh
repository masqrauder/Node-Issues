#!/bin/bash
# Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.

mkdir -p generated

cp ../../port_exposer/target/debug/port_exposer generated/port_exposer

[[ $GITHUB_ACTIONS -eq true ]] && sudo --preserve-env ../../node/ci/free-port-53.sh
docker build -t test_node_image .
