# flo Development Commands
#
#   just              List available commands
#   just build        Build native libflo + reflo
#   just check        Run all checks (format, lint, test, build)
#   just deny         Run cargo-deny license/bans check
#   just audit        Run cargo-audit vulnerability scan
#   just serve        Serve the web demo

scripts := "scripts"

libflo *args:
	@cd {{scripts}} && python3 libflo.py {{args}}

reflo *args:
	@cd {{scripts}} && python3 reflo.py {{args}}

demo *args:
	@cd {{scripts}} && python3 demo.py {{args}}

setup:
	@cd {{scripts}} && python3 libflo.py setup
	@cd {{scripts}} && python3 reflo.py setup

alias fmt := format
format:
	@cd {{scripts}} && python3 libflo.py fmt_check
	@cd {{scripts}} && python3 reflo.py fmt_check

lint:
	@cd {{scripts}} && python3 libflo.py lint
	@cd {{scripts}} && python3 reflo.py lint

test:
	@cd {{scripts}} && python3 libflo.py test
	@cd {{scripts}} && python3 reflo.py test

build:
	@cd {{scripts}} && python3 libflo.py build
	@cd {{scripts}} && python3 reflo.py build

wasm:
	@cd {{scripts}} && python3 libflo.py wasm
	@cd {{scripts}} && python3 reflo.py wasm

install:
	@cd {{scripts}} && python3 reflo.py install

examples:
	@cd {{scripts}} && python3 reflo.py examples

serve port="8080":
	@cd {{scripts}} && python3 demo.py serve {{port}}

clean:
	@cd {{scripts}} && python3 libflo.py clean
	@cd {{scripts}} && python3 reflo.py clean
	@rm -rf Demo/pkg-libflo Demo/pkg-reflo

check:
	@cd {{scripts}} && python3 flo_build.py

deny:
	@cd {{scripts}} && python3 flo_build.py deny

audit:
	@cd {{scripts}} && python3 flo_build.py audit

all: setup check