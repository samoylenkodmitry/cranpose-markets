#!/usr/bin/env bash
# Builds the web bundle into dist/: index.html and the wasm package.
set -euo pipefail

if command -v wasm-pack >/dev/null 2>&1; then
  WASM_PACK="wasm-pack"
elif [[ -x "$HOME/.cargo/bin/wasm-pack" ]]; then
  WASM_PACK="$HOME/.cargo/bin/wasm-pack"
else
  echo "wasm-pack is required. Install it with: cargo install wasm-pack" >&2
  exit 1
fi

BUILD_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/cranpose-markets-web.XXXXXX")"
PKG_STAGE="$BUILD_ROOT/pkg"
DIST_STAGE="$BUILD_ROOT/dist"

"$WASM_PACK" build \
  --target web \
  --release \
  --out-dir "$PKG_STAGE" \
  --no-default-features \
  --features web

mkdir -p "$DIST_STAGE"
cp index.html "$DIST_STAGE/index.html"
cp -R "$PKG_STAGE" "$DIST_STAGE/pkg"
rm -f "$DIST_STAGE/pkg/.gitignore"

for output in pkg dist; do
  if [[ -e "$output" ]]; then
    mv "$output" "$BUILD_ROOT/$output.previous"
  fi
done
mv "$PKG_STAGE" pkg
mv "$DIST_STAGE" dist

echo "Web bundle written to dist/"
