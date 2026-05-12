#!/usr/bin/env python3
"""Generate per-section narration audio from demo/narration.json using edge-tts.

For each segment we render an MP3, probe its duration, then auto-tune `rate`
in a second pass so the audio fits within the segment's `duration_sec` window
(target ≤95% of the window so there's breathing room).

Outputs:
    demo/audio/s1.mp3 ... s6.mp3
    demo/audio/narration_track.mp3 (concatenated full track padded to 3:00)
"""
from __future__ import annotations

import asyncio
import json
import shutil
import subprocess
import sys
from pathlib import Path

import edge_tts

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "demo" / "narration.json"
OUT_DIR = ROOT / "demo" / "audio"
OUT_DIR.mkdir(parents=True, exist_ok=True)

TOTAL_SEC = 180  # 3:00


def ffprobe_duration(p: Path) -> float:
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(p)],
        capture_output=True, text=True, check=True,
    )
    return float(r.stdout.strip())


async def synth(text: str, voice: str, rate: str, pitch: str, out: Path) -> None:
    comm = edge_tts.Communicate(text=text, voice=voice, rate=rate, pitch=pitch)
    await comm.save(str(out))


async def fit_segment(seg: dict, voice: str, base_rate: str, pitch: str) -> Path:
    """Render the segment, then re-render with a faster `rate` if needed."""
    sid = seg["id"]
    target = seg["duration_sec"] * 0.92  # leave 8% headroom for transitions
    out = OUT_DIR / f"{sid}.mp3"

    await synth(seg["text"], voice, base_rate, pitch, out)
    dur = ffprobe_duration(out)
    print(f"  {sid}: pass-1 → {dur:.2f}s (target ≤ {target:.2f}s @ {base_rate})")

    # If too long, ramp up rate in 5% increments up to +35%.
    rate_pct = 0
    while dur > target and rate_pct < 35:
        rate_pct += 5
        rate = f"+{rate_pct}%"
        await synth(seg["text"], voice, rate, pitch, out)
        dur = ffprobe_duration(out)
        print(f"  {sid}: retry @ {rate} → {dur:.2f}s")

    if dur > seg["duration_sec"]:
        print(f"  ⚠  {sid}: still {dur:.2f}s > {seg['duration_sec']}s — narrator may need to pace tighter")
    return out


def build_track(spec: dict, paths: list[Path]) -> Path:
    """Build a single concatenated narration track with silence padding so each
    segment lines up with its `start_sec`."""
    out = OUT_DIR / "narration_track.mp3"
    work = OUT_DIR / "_work"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir()

    segs = spec["segments"]
    inputs = []
    for seg, path in zip(segs, paths):
        seg_path = work / f"{seg['id']}.mp3"
        shutil.copy(path, seg_path)
        seg_dur = ffprobe_duration(seg_path)
        # silence padding to make each segment block exactly its duration_sec long
        pad_sec = max(0.0, seg["duration_sec"] - seg_dur)
        if pad_sec > 0:
            sil = work / f"{seg['id']}_pad.mp3"
            subprocess.run(
                ["ffmpeg", "-y", "-f", "lavfi", "-i", f"anullsrc=r=24000:cl=mono",
                 "-t", f"{pad_sec:.3f}", "-q:a", "9", "-acodec", "libmp3lame", str(sil)],
                check=True, capture_output=True,
            )
            inputs.append(seg_path)
            inputs.append(sil)
        else:
            inputs.append(seg_path)

    # concat list
    list_file = work / "concat.txt"
    with list_file.open("w") as f:
        for p in inputs:
            f.write(f"file '{p}'\n")
    subprocess.run(
        ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(list_file),
         "-c", "copy", str(out)],
        check=True, capture_output=True,
    )
    final_dur = ffprobe_duration(out)
    print(f"\nfinal narration track: {final_dur:.2f}s (target ≈ {TOTAL_SEC}s)")
    return out


async def main() -> None:
    spec = json.loads(SPEC.read_text())
    voice = spec["voice"]
    rate = spec.get("rate", "+0%")
    pitch = spec.get("pitch", "+0Hz")
    print(f"voice: {voice}  rate: {rate}  pitch: {pitch}")

    paths: list[Path] = []
    for seg in spec["segments"]:
        print(f"\n── {seg['id']} {seg['title']} ({seg['duration_sec']}s) ──")
        paths.append(await fit_segment(seg, voice, rate, pitch))

    track = build_track(spec, paths)
    print(f"\n✅ {track}")


if __name__ == "__main__":
    asyncio.run(main())
