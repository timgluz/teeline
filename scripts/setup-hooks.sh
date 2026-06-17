#!/usr/bin/env bash
# Configure local git hooks for teeline.
# Run once after cloning: bash scripts/setup-hooks.sh
set -e

REPO_ROOT=$(git rev-parse --show-toplevel)

git config core.hooksPath .githooks
chmod +x "$REPO_ROOT/.githooks/pre-commit"

echo "Git hooks configured (.githooks/pre-commit is now active)."
echo "Run 'mise install' to ensure all lint tools are available."
