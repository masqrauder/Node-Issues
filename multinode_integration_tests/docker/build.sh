#!/bin/bash
# Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.

function ensure_dns_works_with_github_actions() {
  [[ -d /etc/docker ]] || sudo mkdir -p /etc/docker/
  [[ -f /etc/docker/daemon.json ]] && cat /etc/docker/daemon.json && sudo sed -i -E 's/"dns": .+"/"dns": ["1.1.1.1", "8.8.8.8"]/g' /etc/docker/daemon.json
  sudo service docker restart
}

mkdir -p generated

cp ../../port_exposer/target/debug/port_exposer generated/port_exposer

[[ $GITHUB_ACTIONS -eq true ]] && sudo --preserve-env ../../node/ci/free-port-53.sh
[[ $GITHUB_ACTIONS -eq true ]] && ensure_dns_works_with_github_actions
docker build -t test_node_image .
