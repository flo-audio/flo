import argparse
import shutil

from shared.constants import DEMO_DIR, EXAMPLES_DIR, REFLO_DIR
from shared.format import check, ensure_tool
from shared.run import run

EXAMPLE_SYNTHESIS = [
    ("silence_1sec.flo", ["44100", "1", "16", "trim", "0", "1"]),
    ("white_noise.flo", ["44100", "1", "16", "trim", "0", "1"]),
    ("sine_440hz_mono.flo", ["44100", "1", "16", "synth", "2", "sine", "440"]),
    ("chord_cmajor_stereo.flo", ["44100", "2", "16", "synth", "2", "sine", "261.63", "sine", "329.63", "sine", "392.00"]),
    ("sweep_20_20k.flo", ["44100", "1", "16", "synth", "5", "sq", "20-20000"]),
    ("hires_96khz.flo", ["96000", "1", "16", "synth", "1", "sine", "1000"]),
    ("telephone_8khz.flo", ["8000", "1", "16", "synth", "1", "sine", "1000"]),
    ("click_track_120bpm.flo", ["44100", "1", "16", "synth", "0.05", "sine", "1000"]),
    ("multitone_stereo.flo", ["44100", "2", "16", "synth", "2", "sine", "440", "sine", "880"]),
    ("dtmf_tones.flo", ["44100", "1", "16", "synth", "0.2", "sine", "697", "sine", "1209"]),
]

LOSSY_CHORDS = {
    "lossy_chord_low.flo": ["--lossy", "--quality", "low"],
    "lossy_chord_medium.flo": ["--lossy", "--quality", "medium"],
    "lossy_chord_high.flo": ["--lossy", "--quality", "high"],
    "lossy_chord_veryhigh.flo": ["--lossy", "--quality", "veryhigh"],
    "lossy_chord_transparent.flo": ["--lossy", "--quality", "transparent"],
    "lossy_music_pattern.flo": ["--lossy", "--quality", "high"],
}


def fmt_check():
    check(
        "reflo",
        ["cargo", "fmt", "--check"],
        ["cargo", "fmt"],
        REFLO_DIR,
    )


def lint():
    run(["cargo", "clippy", "--", "-D", "warnings"], REFLO_DIR)


def test():
    run(["cargo", "test"], REFLO_DIR)


def build():
    run(["cargo", "build", "--release"], REFLO_DIR)


def install():
    run(["cargo", "install", "--path", str(REFLO_DIR)], REFLO_DIR)


def wasm():
    ensure_tool("wasm-pack")
    run(
        ["wasm-pack", "build", "--release", "--target", "web", "--features", "wasm"],
        REFLO_DIR,
    )
    shutil.copy(REFLO_DIR / "package.json.template", REFLO_DIR / "pkg" / "package.json")
    dest = DEMO_DIR / "pkg-reflo"
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copytree(REFLO_DIR / "pkg", dest, dirs_exist_ok=True)
    print("[demo] reflo WASM copied to Demo/pkg-reflo/")


def examples():
    ensure_tool("sox")
    run(["cargo", "build"], REFLO_DIR)
    refm = REFLO_DIR / "target" / "debug" / "reflo"

    for name, (rate, channels, bits, *effect) in EXAMPLE_SYNTHESIS:
        wav = f"/tmp/flo_{name.split('.')[0]}.wav"
        run(["sox", "-n", "-r", rate, "-c", channels, "-b", bits, wav, *effect], REFLO_DIR)
        run([str(refm), "encode", wav, str(EXAMPLES_DIR / name)], REFLO_DIR)

    src_wav = EXAMPLES_DIR / "audio.wav"
    run(
        [str(refm), "encode", str(src_wav), str(EXAMPLES_DIR / "audio_lossy.flo"), "--lossy", "--quality", "high"],
        REFLO_DIR,
    )
    run([str(refm), "encode", str(src_wav), str(EXAMPLES_DIR / "audio_lossless.flo")], REFLO_DIR)

    chord_wav = "/tmp/flo_chord_cmajor_stereo.wav"
    for name, extra in LOSSY_CHORDS.items():
        run([str(refm), "encode", chord_wav, str(EXAMPLES_DIR / name), *extra], REFLO_DIR)

    print("[examples] regenerated Examples/*.flo")


def clean():
    run(["cargo", "clean"], REFLO_DIR)


def check_all():
    fmt_check()
    lint()
    test()
    build()


CMD_ALIASES = {
    "setup": lambda: None,
    "build": build,
    "fmt_check": fmt_check,
    "lint": lint,
    "test": test,
    "install": install,
    "wasm": wasm,
    "examples": examples,
    "check": check_all,
    "clean": clean,
}


def main():
    parser = argparse.ArgumentParser(prog="reflo")
    parser.add_argument("command", nargs="?", default="check")
    args = parser.parse_args()
    try:
        CMD_ALIASES[args.command]()
    except KeyError:
        raise SystemExit(f"Unknown command: {args.command}")


if __name__ == "__main__":
    main()