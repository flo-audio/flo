import argparse
import shutil

from shared.constants import DEMO_DIR, LIBFLO_DIR, REPO_ROOT
from shared.format import check, ensure_tool
from shared.run import run


def fmt_check():
    check(
        "libflo",
        ["cargo", "fmt", "--check"],
        ["cargo", "fmt"],
        LIBFLO_DIR,
    )


def lint():
    run(["cargo", "clippy", "--", "-D", "warnings"], LIBFLO_DIR)


def test():
    run(["cargo", "test", "--release", "--all-features"], LIBFLO_DIR)


def test_wasm():
    wasm()
    js_dir = REPO_ROOT / "Tests" / "libflo" / "js"
    run(["npm", "install"], js_dir)
    esm_env = {"NODE_OPTIONS": "--experimental-vm-modules"}
    run(["npm", "test", "--", "--coverage"], js_dir, env=esm_env)


def build():
    run(["cargo", "build", "--release"], LIBFLO_DIR)


def wasm():
    ensure_tool("wasm-pack")
    run(["wasm-pack", "build", "--release", "--target", "web"], LIBFLO_DIR)
    shutil.copy(LIBFLO_DIR / "package.json.template", LIBFLO_DIR / "pkg" / "package.json")
    dest = DEMO_DIR / "pkg-libflo"
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copytree(LIBFLO_DIR / "pkg", dest, dirs_exist_ok=True)
    print("[demo] libflo WASM copied to Demo/pkg-libflo/")


def clean():
    run(["cargo", "clean"], LIBFLO_DIR)


def check_all():
    fmt_check()
    lint()
    test()


CMD_ALIASES = {
    "setup": lambda: run(["rustup", "target", "add", "wasm32-unknown-unknown"], LIBFLO_DIR),
    "build": build,
    "fmt_check": fmt_check,
    "lint": lint,
    "test": test,
    "test_wasm": test_wasm,
    "wasm": wasm,
    "check": check_all,
    "clean": clean,
}


def main():
    parser = argparse.ArgumentParser(prog="libflo")
    parser.add_argument("command", nargs="?", default="check")
    args = parser.parse_args()
    try:
        CMD_ALIASES[args.command]()
    except KeyError:
        raise SystemExit(f"Unknown command: {args.command}")


if __name__ == "__main__":
    main()