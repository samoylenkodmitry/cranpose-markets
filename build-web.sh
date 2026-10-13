#!/usr/bin/env bash
# Builds the web bundle into dist/ with cranpose-build 0.3.0 or later. It
# records the pipelines the first screen draws with in headless Chrome and
# builds the module again with them embedded, so on WebGL the page builds them
# as it starts. Without Chrome it keeps the page it built first.
set -euo pipefail

if ! command -v cranpose-build >/dev/null 2>&1; then
  echo "cranpose-build 0.3.0 or later is required: https://github.com/samoylenkodmitry/cranpose-build/releases" >&2
  exit 1
fi

OUT="target/cranpose-web-page"
cranpose-build build --platform web --release --output-dir "$OUT"
PAGE="$(ls -dt "$OUT"/web/release/build-*/application | head -1)"

if [[ -e dist ]]; then
  rm -rf dist.previous
  mv dist dist.previous
fi
cp -R "$PAGE" dist
rm -f dist/pkg/.gitignore

echo "Web bundle written to dist/"
