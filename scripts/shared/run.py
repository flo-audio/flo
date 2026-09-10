import os
import subprocess
import sys


def run(cmd, cwd, check=True, env=None):
    full_env = os.environ.copy()
    if env:
        full_env.update(env)
    result = subprocess.run(cmd, cwd=cwd, env=full_env)
    if check and result.returncode != 0:
        sys.exit(result.returncode)
    return result.returncode