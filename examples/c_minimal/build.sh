#!/usr/bin/env sh

clang --target=wasm32 --no-standard-libraries -Wl,--export-all -Wl,--no-entry \
  -Wl,--allow-undefined-file=wasm-import.syms \
  -o plugin-c.wasm plugin.c
