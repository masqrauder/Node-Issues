#!/bin/bash -xev
# Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.
CI_DIR="$( cd "$( dirname "$0" )" && pwd )"
TOOLCHAIN_HOME="$1"
source "$CI_DIR"/../../ci/environment.sh "$TOOLCHAIN_HOME"

#export RUSTC_WRAPPER="$HOME/.cargo/bin/sccache"
pushd "$CI_DIR/.."
ci/lint.sh
ci/unit_tests.sh
ci/integration_tests.sh "$TOOLCHAIN_HOME"
popd
