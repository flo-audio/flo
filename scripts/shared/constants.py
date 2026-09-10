from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"
BUILD_ROOT = REPO_ROOT / "Build"
CRATES_DIR = BUILD_ROOT / "crates"
TARGET_DIR = BUILD_ROOT / "target"
LIBFLO_DIR = CRATES_DIR / "libflo"
REFLO_DIR = CRATES_DIR / "reflo"
FIXTURES_DIR = REPO_ROOT / "Tests" / "fixtures"
DEMO_DIR = REPO_ROOT / "Demo"
EXAMPLES_DIR = REPO_ROOT / "Examples"