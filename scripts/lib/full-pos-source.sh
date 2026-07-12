#!/usr/bin/env bash

readonly FULL_POS_SOURCE_NAME="mecab-ko-dic-2.1.1-20180720"
readonly FULL_POS_SOURCE_URL="https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/${FULL_POS_SOURCE_NAME}.tar.gz"
readonly FULL_POS_SOURCE_SHA256="fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

fetch_full_pos_source() {
  local temporary_directory=$1
  local archive="${temporary_directory}/${FULL_POS_SOURCE_NAME}.tar.gz"
  curl --fail --location --silent --show-error --output "${archive}" "${FULL_POS_SOURCE_URL}"

  local actual_source_sha
  actual_source_sha=$(sha256_file "${archive}")
  if [[ "${actual_source_sha}" != "${FULL_POS_SOURCE_SHA256}" ]]; then
    echo "source checksum mismatch: expected ${FULL_POS_SOURCE_SHA256}, got ${actual_source_sha}" >&2
    return 1
  fi

  tar -xzf "${archive}" -C "${temporary_directory}"
  printf '%s\n' "${temporary_directory}/${FULL_POS_SOURCE_NAME}"
}
