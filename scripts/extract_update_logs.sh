#!/usr/bin/env bash
#
# extract_update_logs.sh
# Delegate to the Node.js script to dynamically generate release notes
node "$(dirname "$0")/extract_update_logs.mjs"
