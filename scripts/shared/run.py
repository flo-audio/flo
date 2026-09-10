import subprocess
import sys


def run(cmd, cwd, check=True):
    result = subprocess.run(cmd, cwd=cwd)
    if check and result.returncode != 0:
        sys.exit(result.returncode)
    return result.returncode