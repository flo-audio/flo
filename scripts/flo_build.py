import sys

from libflo import check_all as libflo_check
from reflo import check_all as reflo_check

SURFACES = [
    ("libflo", libflo_check),
    ("reflo", reflo_check),
]


def main():
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


if __name__ == "__main__":
    main()