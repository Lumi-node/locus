#!/usr/bin/env bash
# Combine the Playwright-recorded visual demo with the edge-tts narration
# into a single MP4 ready for YouTube upload.
#
# Inputs:
#   demo-recordings/<dir>/video.webm    (from `npx playwright test`)
#   demo/audio/narration_track.mp3      (from `python3 scripts/generate-narration.py`)
#
# Output:
#   demo/locus-pitch-final.mp4
set -euo pipefail

cd "$(dirname "$0")/.."

WEBM="$(find demo-recordings -name 'video.webm' | head -1)"
AUDIO="demo/audio/narration_track.mp3"
OUT="demo/locus-pitch-final.mp4"

if [[ -z "$WEBM" ]]; then
  echo "no video.webm — run: npx playwright test scripts/demo-solscan.spec.ts" >&2
  exit 1
fi
if [[ ! -f "$AUDIO" ]]; then
  echo "no narration audio — run: python3 scripts/generate-narration.py" >&2
  exit 1
fi

echo ">> video : $WEBM"
echo ">> audio : $AUDIO"
echo ">> out   : $OUT"

# Re-encode video to H.264, mux narration as the audio track, end at shortest stream.
ffmpeg -y \
  -i "$WEBM" \
  -i "$AUDIO" \
  -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p \
  -c:a aac -b:a 192k \
  -map 0:v:0 -map 1:a:0 \
  -shortest \
  -movflags +faststart \
  "$OUT" 2>&1 | tail -8

echo
echo ">> done: $OUT"
ls -lh "$OUT"
ffprobe -v error -show_entries format=duration:format=size -of default=noprint_wrappers=1 "$OUT" 2>&1 | head -3
