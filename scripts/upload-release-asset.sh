#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: upload-release-asset.sh TAG FILE" >&2
  exit 2
fi

tag=$1
file=$2
name=$(basename "${file}")
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

if [[ ! -f "${file}" ]]; then
  echo "release asset is missing: ${file}" >&2
  exit 2
fi

asset_exists=$(gh release view "${tag}" \
  --json assets \
  --jq ".assets[] | select(.name == \"${name}\") | .name")
if [[ -n "${asset_exists}" ]]; then
  gh release download "${tag}" --pattern "${name}" \
    --dir "${temporary_directory}"
  if ! cmp --silent "${file}" "${temporary_directory}/${name}"; then
    echo "release asset already exists with different contents: ${name}" >&2
    exit 1
  fi
  echo "release asset already matches: ${name}"
  exit 0
fi

gh release upload "${tag}" "${file}"
