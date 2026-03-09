#!/bin/bash
# Regenerate example flo files from source audio

set -e

cd "$(dirname "$0")/.."

source "$HOME/.cargo/env"

REFLO="./reflo/target/debug/reflo"

echo "Generating example files..."

# Lossless examples
echo "  silence_1sec.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/silence.wav trim 0 1
$REFLO encode /tmp/silence.wav Examples/silence_1sec.flo

echo "  white_noise.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/noise.wav trim 0 1
$REFLO encode /tmp/noise.wav Examples/white_noise.flo

echo "  sine_440hz_mono.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/sine.wav synth 2 sine 440
$REFLO encode /tmp/sine.wav Examples/sine_440hz_mono.flo

echo "  chord_cmajor_stereo.flo..."
sox -n -r 44100 -c 2 -b 16 /tmp/chord.wav synth 2 sine 261.63 sine 329.63 sine 392.00
$REFLO encode /tmp/chord.wav Examples/chord_cmajor_stereo.flo

echo "  sweep_20_20k.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/sweep.wav synth 5 sq 20-20000
$REFLO encode /tmp/sweep.wav Examples/sweep_20_20k.flo

echo "  hires_96khz.flo..."
sox -n -r 96000 -c 1 -b 16 /tmp/hires.wav synth 1 sine 1000
$REFLO encode /tmp/hires.wav Examples/hires_96khz.flo

echo "  telephone_8khz.flo..."
sox -n -r 8000 -c 1 -b 16 /tmp/tel.wav synth 1 sine 1000
$REFLO encode /tmp/tel.wav Examples/telephone_8khz.flo

echo "  click_track_120bpm.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/click.wav synth 0.05 sine 1000
$REFLO encode /tmp/click.wav Examples/click_track_120bpm.flo

echo "  multitone_stereo.flo..."
sox -n -r 44100 -c 2 -b 16 /tmp/multi.wav synth 2 sine 440 sine 880
$REFLO encode /tmp/multi.wav Examples/multitone_stereo.flo

echo "  dtmf_tones.flo..."
sox -n -r 44100 -c 1 -b 16 /tmp/dtmf.wav synth 0.2 sine 697 sine 1209
$REFLO encode /tmp/dtmf.wav Examples/dtmf_tones.flo

# Lossy examples
echo "  audio_lossy.flo..."
$REFLO encode Examples/audio.wav Examples/audio_lossy.flo --lossy --quality high

echo "  lossy_chord_low.flo..."
$REFLO encode /tmp/chord.wav Examples/lossy_chord_low.flo --lossy --quality low

echo "  lossy_chord_medium.flo..."
$REFLO encode /tmp/chord.wav Examples/lossy_chord_medium.flo --lossy --quality medium

echo "  lossy_chord_high.flo..."
$REFLO encode /tmp/chord.wav Examples/lossy_chord_high.flo --lossy --quality high

echo "  lossy_chord_veryhigh.flo..."
$REFLO encode /tmp/chord.wav Examples/lossy_chord_veryhigh.flo --lossy --quality veryhigh

echo "  lossy_chord_transparent.flo..."
$REFLO encode /tmp/chord.wav Examples/lossy_chord_transparent.flo --lossy --quality transparent

echo "  lossy_music_pattern.flo..."
$REFLO encode /tmp/multi.wav Examples/lossy_music_pattern.flo --lossy --quality high

echo "  audio_lossless.flo..."
$REFLO encode Examples/audio.wav Examples/audio_lossless.flo

echo ""
echo "Verifying files..."
for f in Examples/*.flo; do
    $REFLO info "$f" 2>/dev/null | grep -E "Duration|Frames" | head -1
done
