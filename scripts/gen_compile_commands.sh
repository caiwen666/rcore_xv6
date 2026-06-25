#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT/user"
bear -- make clean all
cp compile_commands.json "$ROOT/compile_commands.json"
