import argparse
import sys

from shared.constants import BUILD_ROOT
from shared.format import ensure_tool
from shared.run import run
from libflo import check_all as libflo_check
from reflo import check_all as reflo_check

SURFACES = [
    ("libflo", libflo_check),
    ("reflo", reflo_check),
]


def check_all():
    failed = []
    for name, check in SURFACES:
        print(f"[check] {name}...")
        try:
            check()
        except SystemExit:
            print(f"[FAIL] {name}")
            failed.append(name)
            continue
        print(f"[PASS] {name}")
    if failed:
        print(f"[check] failed: {', '.join(failed)}")
        sys.exit(1)
    print("[check] all surfaces passed")


def deny():
    ensure_tool("cargo-deny")
    run(["cargo", "deny", "--manifest-path", str(BUILD_ROOT / "Cargo.toml"), "check"], BUILD_ROOT)


def audit():
    ensure_tool("cargo-audit")
    run(["cargo", "audit"], BUILD_ROOT)


COMMANDS = {
    "check": check_all,
    "deny": deny,
    "audit": audit,
}


def main():
    parser = argparse.ArgumentParser(prog="flo_build")
    parser.add_argument("command", nargs="?", default="check")
    args = parser.parse_args()
    try:
        COMMANDS[args.command]()
    except KeyError:
        raise SystemExit(f"Unknown command: {args.command}")


if __name__ == "__main__":
    main()