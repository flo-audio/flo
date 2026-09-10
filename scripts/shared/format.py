import os
import shutil
import subprocess

from .run import run


def check(label, check_cmd, fix_cmd, cwd):
    if os.environ.get("CI"):
        run(check_cmd, cwd)
        return
    result = subprocess.run(check_cmd, cwd=cwd, capture_output=True)
    if result.returncode != 0:
        print(f"[fmt] {label}: formatting issues found, running fix...")
        run(fix_cmd, cwd)
    else:
        print(f"[fmt] {label}: OK")


def ensure_tool(name):
    if shutil.which(name):
        return
    raise SystemExit(f"[setup] '{name}' not found on PATH. Install it and try again.")