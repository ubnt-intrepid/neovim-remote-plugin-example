#!/bin/bash

plugin_root=$(cd $(dirname $BASH_SOURCE); pwd)

pushd $plugin_root > /dev/null

cargo run -q

popd > /dev/null
