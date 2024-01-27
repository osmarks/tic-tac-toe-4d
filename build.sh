#!/bin/sh
wasm-pack build --target=web
sed -i "s/import.meta.url/location/" pkg/tic_tac_toe_4d.js
ESBUILD_FLAGS="--sourcemap --loader:.wasm=copy --minify"
npx esbuild --bundle src/app.js --outdir=dist $ESBUILD_FLAGS
npx esbuild --bundle src/webworker_glue.js --outdir=dist $ESBUILD_FLAGS
cp index.html dist
cp pkg/*.wasm dist
cp -r assets/ dist