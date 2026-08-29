#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: publish-versioned-site.sh VERSION ARCHIVE INDEX" >&2
  exit 2
fi

version=$1
archive=$2
index=$3
site_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
repo_root=$(cd "${site_dir}/.." && pwd)
bucket=kfind-assets
object_prefix="site/versions/${version}"
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

if [[ ! "${version}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?$ ]]; then
  echo "invalid site version: ${version}" >&2
  exit 2
fi
for path in "${archive}" "${index}"; do
  if [[ ! -f "${path}" ]]; then
    echo "versioned site artifact is missing: ${path}" >&2
    exit 2
  fi
done

download_optional_object() {
  local key=$1
  local output=$2
  local error_output="${temporary_directory}/wrangler-error.txt"
  if wrangler r2 object get "${key}" \
    --file "${output}" --remote 2>"${error_output}"; then
    return 0
  fi
  if grep -Fq 'The specified key does not exist.' "${error_output}"; then
    return 1
  fi
  cat "${error_output}" >&2
  exit 1
}

remote_index="${temporary_directory}/index.json"
remote_index_exists=0
if download_optional_object \
  "${bucket}/${object_prefix}/index.json" "${remote_index}"; then
  remote_index_exists=1
  if ! cmp --silent "${index}" "${remote_index}"; then
    echo "versioned site ${version} already exists with a different index" >&2
    exit 1
  fi
fi

remote_archive="${temporary_directory}/site.tar"
if download_optional_object \
  "${bucket}/${object_prefix}/site.tar" "${remote_archive}"; then
  if ! cmp --silent "${archive}" "${remote_archive}"; then
    echo "versioned site ${version} already exists with a different archive" >&2
    exit 1
  fi
else
  wrangler r2 object put "${bucket}/${object_prefix}/site.tar" \
    --file "${archive}" \
    --content-type application/x-tar \
    --cache-control 'public, max-age=31536000, immutable' \
    --remote \
    --force
fi
if [[ "${remote_index_exists}" == 0 ]]; then
  wrangler r2 object put "${bucket}/${object_prefix}/index.json" \
    --file "${index}" \
    --content-type 'application/json; charset=utf-8' \
    --cache-control 'public, max-age=31536000, immutable' \
    --remote \
    --force
fi

existing_manifest="${temporary_directory}/manifest-existing.json"
manifest="${temporary_directory}/manifest.json"
manifest_arguments=()
if download_optional_object \
  "${bucket}/site/versions/manifest.json" "${existing_manifest}"; then
  manifest_arguments+=(--existing "${existing_manifest}")
fi
python3 "${repo_root}/tools/release/site_archive.py" update-manifest \
  --version "${version}" \
  "${manifest_arguments[@]}" \
  --output "${manifest}"
wrangler r2 object put "${bucket}/site/versions/manifest.json" \
  --file "${manifest}" \
  --content-type 'application/json; charset=utf-8' \
  --cache-control no-cache \
  --remote \
  --force
