#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail() { printf 'release policy: %s\n' "$1" >&2; exit 1; }

[[ -f docs/release/RELEASE-MANIFEST.toml ]] || fail 'manifest missing'
[[ -f docs/release/SBOM.tsv ]] || fail 'SBOM missing'
[[ -f docs/release/LICENSES.md ]] || fail 'licence notice missing'
[[ -f docs/release/WAIVERS.md ]] || fail 'waiver report missing'

grep -q '^external_documents_archived = false$' docs/SOURCES.toml \
    || fail 'external source archival policy changed without review'
grep -q '^r2_sources_manifest_sha256 = "[0-9a-f]\{64\}"$' \
    docs/release/RELEASE-MANIFEST.toml || fail 'R2 hash missing or malformed'
[[ "$(head -1 docs/release/SBOM.tsv)" == $'name\tversion\tlicense\tsource\tchecksum' ]] \
    || fail 'SBOM header malformed'

awk -F '\t' 'NR > 1 && NF != 5 { bad = 1 } END { exit bad }' docs/release/SBOM.tsv \
    || fail 'SBOM contains malformed rows'
if rg -n '/home/|/Users/|[A-Za-z]:\\\\|file:///' docs/release; then
    fail 'release dossier contains an absolute host path'
fi

question_count="$(rg -c '^[0-9]+\. ' docs/OPEN_QUESTIONS.md || true)"
((question_count <= 10)) || fail 'more than ten open questions'
if rg -n '^[0-9]+\. ' docs/OPEN_QUESTIONS.md | grep -v 'Différable'; then
    fail 'an open question is still blocking without an explicit decision'
fi

printf 'release policy: PASS (dossier, source policy, SBOM shape, paths, waivers, open questions)\n'
