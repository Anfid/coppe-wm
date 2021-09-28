.PHONY: build
build:
	@clang --target=wasm32 --no-standard-libraries \
	-Wl,--export-all -Wl,--no-entry \
	-Wl,--allow-undefined-file=libcoppe/wasm-import.syms \
	-o plugin.wasm plugin.c

.PHONY: release
release:
	@clang -O2 --target=wasm32 --no-standard-libraries \
	-Wl,--export-all -Wl,--no-entry \
	-Wl,--allow-undefined-file=libcoppe/wasm-import.syms \
	-o plugin.wasm plugin.c

