#!/usr/bin/env bash

export RES_PATH=learning-wgpu

#

cargo build --no-default-features --profile wasm-release --target wasm32-unknown-unknown --features webgl \
	--bin tut01-window

mkdir -p "docs/public/wasm"

for i in target/wasm32-unknown-unknown/wasm-release/*.wasm;
do
	wasm-bindgen --no-typescript --out-dir wasm --web "$i";
	filename=$(basename "$i");
	name_no_extension="${filename%.wasm}";
	wasm-opt -Oz wasm/"$name_no_extension"_bg.wasm --output docs/public/wasm/"$name_no_extension"_bg.wasm;

	cp wasm/"$name_no_extension".js docs/public/wasm/"$name_no_extension".js
done

mkdir -p docs/public/assets

cp -r code/integration-and-debugging/wgpu_in_web/assets/* docs/public/assets/

