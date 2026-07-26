#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

node tools/validate-il2cpp-building-metadata.mjs
python3 tools/validate-serialized-building-tables.py
python3 tools/validate-core-economy-tables.py
node tools/validate-building-asset-evidence.mjs reverse-engineering/evidence/building-asset-evidence-v1.json
python3 tools/validate-building-capability-contract.py
node tools/validate-building-ui-contract.mjs
python3 tools/validate-building-condition-evidence.py
python3 tools/validate-building-town-geometry.py
python3 tools/validate-building-skin-evidence.py
node tools/validate-building-registry.mjs packages/content/releases/evil-hunter-1.411/building-registry.json
