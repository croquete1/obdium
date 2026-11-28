#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
check="$root/scripts/check-native-deps.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
tree="$tmp/tree.txt"

reject() {
    if bash "$check" "$1" "$2" >/dev/null 2>&1; then
        echo "Unexpectedly accepted: $1 $2" >&2
        exit 1
    fi
}

printf '%s\n' 'obdium-core-probe v0.1.0' 'serialport v4.10.1' 'slintlike v1.0.0' > "$tree"
bash "$check" core "$tree"
reject native "$tree"
printf '%s\n' 'slint v1.19.1' 'i-slint-core v1.19.1' >> "$tree"
bash "$check" native "$tree"
reject core "$tree"

for package in tauri tauri-runtime tauri-plugin-shell wry webkit2gtk-sys webview2-com-sys javascriptcore-rs-sys; do
    printf '%s\n' 'obdium-core-probe v0.1.0' 'slint v1.19.1' "$package v1.0.0" > "$tree"
    reject native "$tree"
    printf '%s\n' 'obdium-core-probe v0.1.0' "$package v1.0.0" > "$tree"
    reject core "$tree"
done
for package in slint-build i-slint-core; do
    printf '%s\n' 'obdium-core-probe v0.1.0' "$package v1.0.0" > "$tree"
    reject core "$tree"
done
printf '%s\n' 'serialport v4.10.1' > "$tree"
reject core "$tree"
reject native "$tree"
: > "$tree"
reject core "$tree"
reject native "$tree"
reject core "$tmp/missing"
reject native "$tmp/missing"
echo 'Native dependency boundary fixtures passed'
