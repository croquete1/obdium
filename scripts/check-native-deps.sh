#!/usr/bin/env bash
# Check the normal/build cargo tree of the native experiments.
set -euo pipefail

if [[ $# -ne 2 || ( "$1" != core && "$1" != native ) ]]; then
    echo 'Usage: check-native-deps.sh core|native CARGO_TREE_FILE' >&2
    exit 2
fi
mode=$1
tree=$2
if [[ ! -f "$tree" || ! -s "$tree" ]]; then
    echo 'Missing or empty dependency tree' >&2
    exit 2
fi

# Match whole package families, including bindings and runtime subcrates.
forbidden='^(tauri|wry|webkit2gtk|webview2-com|javascriptcore-rs)([-[:space:]]|$)'
if [[ "$mode" == core ]]; then
    forbidden='^(tauri|wry|webkit2gtk|webview2-com|javascriptcore-rs|slint|i-slint)([-[:space:]]|$)'
fi

if grep -E "$forbidden" "$tree"; then
    echo 'Forbidden GUI or legacy WebView dependency' >&2
    exit 1
else
    status=$?
    if [[ "$status" -ne 1 ]]; then
        exit "$status"
    fi
fi

grep -qE '^obdium-core-probe v' "$tree"
if [[ "$mode" == native ]]; then
    grep -qE '^slint v' "$tree"
fi
