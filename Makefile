# installation root
ROOTDIR ?= 
# installation prefix
PREFIX ?= /usr/local

DESTDIR := $(ROOTDIR)$(PREFIX)

bindir := $(DESTDIR)/bin
sharedir := $(DESTDIR)/share

RUST_LOG ?= coppe_wm=info
export RUST_LOG

help:
	@echo "Available targets:"
	@echo " help - Print available targets"
	@echo " build - Build WM"
	@echo " build-release - Build WM in release mode"
	@echo " install - Install. Available variables: ROOTDIR, PREFIX"
	@echo " uninstall - Uninstall. Available variables: ROOTDIR, PREFIX"
	@echo " xephyr - Run WM inside Xephyr"

build:
	cargo build

build-release:
	cargo build --release

installdirs:
	install -d \
		$(bindir) \
		$(sharedir)/xsessions

install: installdirs
	@echo Installing to $(DESTDIR)
	install -m 0755 target/release/coppe-wm $(bindir)/coppe-wm
	install -m 0644 configuration/xsessions/coppe-wm.desktop $(sharedir)/xsessions/coppe-wm.desktop

uninstall:
	@echo Uninstalling from $(DESTDIR)
	rm -rf \
		$(bindir)/coppe-wm \
		$(sharedir)/xsessions/coppe-wm.desktop

xephyr: build
	Xephyr -name 'coppe-wm' -br -ac -noreset -screen 1024x768 +xinerama :2 &
	@sleep 1
	DISPLAY=:2 exec target/debug/coppe-wm

.PHONY: help build build-release installdirs install uninstall xephyr
