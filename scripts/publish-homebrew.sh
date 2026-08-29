#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: publish-homebrew.sh VERSION FORMULA" >&2
  exit 2
fi

version=$1
formula=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
tap_repository=SeokminHong/homebrew-brew
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

if [[ ! "${version}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?$ ]]; then
  echo "invalid Homebrew version: ${version}" >&2
  exit 2
fi
if [[ ! -f "${formula}" ]]; then
  echo "Homebrew formula is missing: ${formula}" >&2
  exit 2
fi
if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "TAP_GITHUB_TOKEN is required" >&2
  exit 2
fi

main() {
  gh auth setup-git
  gh repo clone "${tap_repository}" "${temporary_directory}/tap" -- --quiet
  cd "${temporary_directory}/tap"

  if formula_is_published Formula/kfind.rb; then
    echo "Homebrew formula ${version} is already published with bottles."
    exit 0
  fi

  pull_request=$(gh pr list \
    --repo "${tap_repository}" \
    --state open \
    --json number,title \
    --jq ".[] | select(.title == \"kfind ${version}\") | .number" | head -1)

  if [[ -z "${pull_request}" ]]; then
    branch="kfind-v${version}-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"
    git switch --create "${branch}" origin/main
    install -m 0644 "${formula}" Formula/kfind.rb
    git config user.name "github-actions[bot]"
    git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    git add Formula/kfind.rb
    git commit -m "kfind ${version}"
    git push --set-upstream origin "${branch}"

    body="${temporary_directory}/pull-request.md"
    {
      printf '## 변경 사항\n\n'
      printf -- '- kfind %s 배포\n\n' "${version}"
      printf '## 검증\n\n'
      printf -- '- %s\n' "\`brew test-bot --only-formulae\`"
    } >"${body}"
    pull_request=$(gh pr create \
      --repo "${tap_repository}" \
      --base main \
      --head "${branch}" \
      --title "kfind ${version}" \
      --body-file "${body}")
    pull_request=${pull_request##*/}
  else
    branch=$(gh pr view "${pull_request}" \
      --repo "${tap_repository}" \
      --json headRefName \
      --jq .headRefName)
    git fetch --quiet origin "refs/heads/${branch}:refs/remotes/origin/${branch}"
    git show "origin/${branch}:Formula/kfind.rb" \
      >"${temporary_directory}/open-pr-kfind.rb"
    if ! cmp --silent \
      "${formula}" "${temporary_directory}/open-pr-kfind.rb"; then
      echo "open Homebrew pull request contains a different formula" >&2
      exit 1
    fi
  fi

  wait_for_checks "${pull_request}"
  verify_bottle_artifacts "${pull_request}"

  if gh pr view "${pull_request}" \
    --repo "${tap_repository}" \
    --json labels \
    --jq '.labels[].name' | grep -Fqx pr-pull; then
    label_time=1970-01-01T00:00:00Z
  else
    label_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    gh pr edit "${pull_request}" --repo "${tap_repository}" --add-label pr-pull
  fi
  publish_run=$(wait_for_publish_run "${label_time}")
  gh run watch "${publish_run}" --repo "${tap_repository}" --exit-status

  for _ in $(seq 1 60); do
    git fetch --quiet origin main
    git show origin/main:Formula/kfind.rb >"${temporary_directory}/published-kfind.rb"
    if formula_is_published "${temporary_directory}/published-kfind.rb"; then
      echo "Homebrew formula ${version} and bottles are published."
      exit 0
    fi
    sleep 5
  done

  echo "Homebrew formula ${version} was not published to tap main" >&2
  exit 1
}

formula_is_published() {
  local path=$1
  [[ -f "${path}" ]] &&
    grep -Fq "/download/v${version}/" "${path}" &&
    grep -Fq '  bottle do' "${path}"
}

wait_for_checks() {
  local number=$1
  local count
  for _ in $(seq 1 60); do
    count=$(gh pr view "${number}" \
      --repo "${tap_repository}" \
      --json statusCheckRollup \
      --jq '.statusCheckRollup | length')
    if ((count > 0)); then
      gh pr checks "${number}" \
        --repo "${tap_repository}" \
        --watch \
        --fail-fast
      return
    fi
    sleep 5
  done
  echo "Homebrew test-bot checks did not start" >&2
  exit 1
}

verify_bottle_artifacts() {
  local number=$1
  local head_sha
  local run_id
  local artifacts
  head_sha=$(gh pr view "${number}" \
    --repo "${tap_repository}" \
    --json headRefOid \
    --jq .headRefOid)
  run_id=$(gh run list \
    --repo "${tap_repository}" \
    --workflow tests.yml \
    --event pull_request \
    --json databaseId,headSha,conclusion \
    --jq ".[] | select(.headSha == \"${head_sha}\" and .conclusion == \"success\") | .databaseId" \
    | head -1)
  if [[ -z "${run_id}" ]]; then
    echo "successful Homebrew test-bot run was not found" >&2
    exit 1
  fi
  artifacts=$(gh api "repos/${tap_repository}/actions/runs/${run_id}/artifacts")
  for name in bottles_macos-15 bottles_macos-26; do
    if ! jq -e --arg name "${name}" \
      '.artifacts[] | select(.name == $name and .expired == false and .size_in_bytes > 0)' \
      <<<"${artifacts}" >/dev/null; then
      echo "required Homebrew bottle artifact is missing: ${name}" >&2
      exit 1
    fi
  done
}

wait_for_publish_run() {
  local created_after=$1
  local run_id
  for _ in $(seq 1 60); do
    run_id=$(gh run list \
      --repo "${tap_repository}" \
      --workflow publish.yml \
      --event pull_request_target \
      --json databaseId,createdAt,displayTitle \
      --jq ".[] | select(.createdAt >= \"${created_after}\" and .displayTitle == \"kfind ${version}\") | .databaseId" \
      | head -1)
    if [[ -n "${run_id}" ]]; then
      printf '%s\n' "${run_id}"
      return
    fi
    sleep 5
  done
  echo "Homebrew pr-pull workflow did not start" >&2
  exit 1
}

main
