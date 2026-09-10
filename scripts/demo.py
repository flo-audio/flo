import argparse

from shared.constants import DEMO_DIR
from shared.run import run


def build():
    from libflo import wasm as libflo_wasm
    from reflo import wasm as reflo_wasm

    libflo_wasm()
    reflo_wasm()
    print("[demo] WASM packages ready: Demo/pkg-libflo/, Demo/pkg-reflo/")


def serve(port):
    run(["python3", "-m", "http.server", str(port)], DEMO_DIR)


def check_all():
    build()


CMD_ALIASES = {
    "build": build,
    "serve": serve,
    "check": check_all,
}


def main():
    parser = argparse.ArgumentParser(prog="demo")
    parser.add_argument("command", nargs="?", default="check")
    parser.add_argument("positional", nargs="*")
    args = parser.parse_args()
    try:
        if args.command == "serve":
            CMD_ALIASES["serve"](int(args.positional[0]) if args.positional else 8080)
        else:
            if args.positional:
                raise SystemExit(f"Unexpected arguments: {' '.join(args.positional)}")
            CMD_ALIASES[args.command]()
    except KeyError:
        raise SystemExit(f"Unknown command: {args.command}")


if __name__ == "__main__":
    main()