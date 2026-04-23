#!/usr/bin/env bash

set -euo pipefail

CURRENT_REF="${1:-${GITHUB_REF_NAME:-}}"
OUTPUT_FILE="${2:-release-notes.md}"
INCLUDE_MERGES="${INCLUDE_MERGES:-false}"

if [[ -z "${CURRENT_REF}" ]]; then
  echo "Usage: $0 <tag-or-ref> [output-file]" >&2
  exit 1
fi

if ! git rev-parse --verify "${CURRENT_REF}^{commit}" >/dev/null 2>&1; then
  echo "Reference '${CURRENT_REF}' does not exist." >&2
  exit 1
fi

PREVIOUS_TAG="$(git describe --tags --abbrev=0 "${CURRENT_REF}^" 2>/dev/null || true)"
RANGE="${CURRENT_REF}"
if [[ -n "${PREVIOUS_TAG}" ]]; then
  RANGE="${PREVIOUS_TAG}..${CURRENT_REF}"
fi

FEAT_ITEMS=""
FIX_ITEMS=""
PERF_ITEMS=""
REFACTOR_ITEMS=""
DOCS_ITEMS=""
BUILD_ITEMS=""
CI_ITEMS=""
TEST_ITEMS=""
CHORE_ITEMS=""
REVERT_ITEMS=""
OTHER_ITEMS=""
declare -a BREAKING_ITEMS=()

LOG_ARGS=(--reverse --pretty=format:'%H%x1f%s%x1f%b%x1e')
if [[ "${INCLUDE_MERGES}" != "true" ]]; then
  LOG_ARGS+=(--no-merges)
fi

append_section_item() {
  local section="$1"
  local item="$2"
  case "${section}" in
    feat) FEAT_ITEMS+="${item}"$'\n' ;;
    fix) FIX_ITEMS+="${item}"$'\n' ;;
    perf) PERF_ITEMS+="${item}"$'\n' ;;
    refactor) REFACTOR_ITEMS+="${item}"$'\n' ;;
    docs) DOCS_ITEMS+="${item}"$'\n' ;;
    build) BUILD_ITEMS+="${item}"$'\n' ;;
    ci) CI_ITEMS+="${item}"$'\n' ;;
    test) TEST_ITEMS+="${item}"$'\n' ;;
    chore) CHORE_ITEMS+="${item}"$'\n' ;;
    revert) REVERT_ITEMS+="${item}"$'\n' ;;
    *) OTHER_ITEMS+="${item}"$'\n' ;;
  esac
}

print_section() {
  local title="$1"
  local items="$2"
  if [[ -z "${items}" ]]; then
    return 1
  fi
  echo
  echo "### ${title}"
  printf '%s' "${items}"
  return 0
}

while IFS= read -r -d $'\x1e' record; do
  record="${record#$'\n'}"
  [[ -z "${record}" ]] && continue

  if [[ "${record}" != *$'\x1f'* ]]; then
    continue
  fi
  sha="${record%%$'\x1f'*}"
  rest="${record#*$'\x1f'}"
  if [[ "${rest}" != *$'\x1f'* ]]; then
    continue
  fi
  subject="${rest%%$'\x1f'*}"
  body="${rest#*$'\x1f'}"
  [[ -z "${sha}" || -z "${subject}" ]] && continue

  section="other"
  description="${subject}"
  is_breaking="false"
  breaking_note=""
  conventional_type=""

  if [[ "${subject}" == *": "* ]]; then
    prefix="${subject%%: *}"
    suffix="${subject#*: }"
    type_candidate="${prefix}"

    if [[ "${type_candidate}" == *"!" ]]; then
      is_breaking="true"
      type_candidate="${type_candidate%!}"
    fi

    if [[ "${type_candidate}" == *"("*")" ]]; then
      type_candidate="${type_candidate%%(*}"
    fi

    if [[ -n "${type_candidate}" && "${type_candidate}" != *[![:alpha:]]* ]]; then
      conventional_type="$(printf '%s' "${type_candidate}" | tr '[:upper:]' '[:lower:]')"
      description="${suffix}"
    fi
  fi

  case "${conventional_type}" in
    feat|feature) section="feat" ;;
    fix|bugfix) section="fix" ;;
    perf) section="perf" ;;
    refactor) section="refactor" ;;
    docs|doc) section="docs" ;;
    build) section="build" ;;
    ci) section="ci" ;;
    test|tests) section="test" ;;
    chore) section="chore" ;;
    revert) section="revert" ;;
    "") section="other" ;;
    *) section="other" ;;
  esac

  while IFS= read -r line; do
    case "${line}" in
      BREAKING\ CHANGE:*)
        is_breaking="true"
        breaking_note="${line#BREAKING CHANGE:}"
        breaking_note="${breaking_note# }"
        break
        ;;
    esac
  done <<< "${body}"

  short_sha="${sha:0:7}"
  commit_ref="\`${short_sha}\`"
  if [[ -n "${GITHUB_SERVER_URL:-}" && -n "${GITHUB_REPOSITORY:-}" ]]; then
    commit_ref="[\`${short_sha}\`](${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/commit/${sha})"
  fi

  append_section_item "${section}" "- ${description} (${commit_ref})"

  if [[ "${is_breaking}" == "true" ]]; then
    if [[ -n "${breaking_note}" ]]; then
      BREAKING_ITEMS+=("- ${breaking_note} (${commit_ref})")
    else
      BREAKING_ITEMS+=("- ${description} (${commit_ref})")
    fi
  fi
done < <(git log "${LOG_ARGS[@]}" "${RANGE}" && printf '\x1e')

{
  echo "## What's Changed"
  if [[ -n "${PREVIOUS_TAG}" ]]; then
    echo
    echo "_Changes since ${PREVIOUS_TAG}_"
  fi

  if (( ${#BREAKING_ITEMS[@]} > 0 )); then
    echo
    echo "### Breaking Changes"
    printf '%s\n' "${BREAKING_ITEMS[@]}"
  fi

  has_section_items="false"
  if print_section "Features" "${FEAT_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Fixes" "${FIX_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Performance" "${PERF_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Refactor" "${REFACTOR_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Documentation" "${DOCS_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Build" "${BUILD_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "CI" "${CI_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Tests" "${TEST_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Chores" "${CHORE_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Reverts" "${REVERT_ITEMS}"; then
    has_section_items="true"
  fi
  if print_section "Other Changes" "${OTHER_ITEMS}"; then
    has_section_items="true"
  fi

  if [[ "${has_section_items}" != "true" ]]; then
    echo
    echo "- No commits found in range ${RANGE}."
  fi

  if [[ -n "${PREVIOUS_TAG}" && -n "${GITHUB_SERVER_URL:-}" && -n "${GITHUB_REPOSITORY:-}" ]]; then
    echo
    echo "**Full Changelog**: [${PREVIOUS_TAG}...${CURRENT_REF}](${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/compare/${PREVIOUS_TAG}...${CURRENT_REF})"
  fi
} > "${OUTPUT_FILE}"

echo "Generated ${OUTPUT_FILE} from range ${RANGE}"
