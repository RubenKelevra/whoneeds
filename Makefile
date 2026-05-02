INSTALL = install
INSTALL_PROGRAM = $(INSTALL) -Dm755
PREFIX ?= /usr
BINDIR = $(PREFIX)/bin
CARGO ?= cargo

.PHONY: all build check install clean

all: build

build:
	$(CARGO) build --release --locked

check:
	$(CARGO) fmt --all -- --check
	$(CARGO) test
	$(CARGO) clippy --all-targets --all-features -- -D warnings -D unsafe_code -D clippy::pedantic

install: build
	$(INSTALL_PROGRAM) target/release/whoneeds $(DESTDIR)$(BINDIR)/whoneeds

clean:
	$(CARGO) clean
