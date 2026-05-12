#!/usr/bin/env python3
"""Generate narration audio with Sesame CSM-1B — much more natural than edge-tts.

CSM is a 1B-param conversational speech model. The single speaker we use here
("0") is the default CSM voice — clean, calm, podcast-grade.

Outputs:
    demo/audio/csm_s1.wav … csm_s6.wav   (24 kHz mono)
    demo/audio/narration_track.mp3       (concatenated, padded to 180s)
"""
from __future__ import annotations

import json
import subprocess
import sys
import shutil
from pathlib import Path

import torch
import torchaudio
from transformers import CsmForConditionalGeneration, AutoProcessor

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "demo" / "narration.json"
OUT = ROOT / "demo" / "audio"
OUT.mkdir(parents=True, exist_ok=True)

MODEL_ID = "sesame/csm-1b"
SPEAKER = "0"  # default CSM speaker
TOTAL_SEC = 180


def ffprobe_duration(p: Path) -> float:
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(p)],
        capture_output=True, text=True, check=True,
    )
    return float(r.stdout.strip())


def synthesize(model, processor, text: str, device: str, out_path: Path) -> None:
    """Generate audio for `text` and save directly via processor.save_audio."""
    conversation = [
        {"role": SPEAKER, "content": [{"type": "text", "text": text}]},
    ]
    inputs = processor.apply_chat_template(
        conversation, tokenize=True, return_dict=True, return_tensors="pt",
    ).to(device)
    with torch.no_grad():
        audio = model.generate(
            **inputs,
            output_audio=True,
            max_new_tokens=1500,
        )
    processor.save_audio(audio, str(out_path))


def main() -> None:
    spec = json.loads(SPEC.read_text())
    segs = spec["segments"]

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"device: {device}")
    dtype = torch.bfloat16 if device == "cuda" else torch.float32

    print(f"loading {MODEL_ID}")
    processor = AutoProcessor.from_pretrained(MODEL_ID)
    model = CsmForConditionalGeneration.from_pretrained(
        MODEL_ID, torch_dtype=dtype,
    ).to(device)
    model.eval()
    print(f"loaded {MODEL_ID}")

    seg_paths = []
    for seg in segs:
        print(f"\n── {seg['id']} {seg['title']} ({seg['duration_sec']}s) ──")
        text = seg["text"]
        print(f"  text: {text[:80]}…")
        out = OUT / f"csm_{seg['id']}.wav"
        synthesize(model, processor, text, device, out)
        dur = ffprobe_duration(out)
        print(f"  → {out.name}  {dur:.2f}s")
        seg_paths.append((seg, out))

    # Concat into one track with silence padding so each segment lands in its window.
    work = OUT / "_work_csm"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir()

    pieces = []
    for seg, p in seg_paths:
        dur = ffprobe_duration(p)
        pieces.append(p)
        pad_sec = max(0.0, seg["duration_sec"] - dur)
        if pad_sec > 0.05:
            sil = work / f"sil_{seg['id']}.wav"
            subprocess.run(
                ["ffmpeg", "-y", "-f", "lavfi",
                 "-i", "anullsrc=r=24000:cl=mono",
                 "-t", f"{pad_sec:.3f}",
                 "-c:a", "pcm_s16le", str(sil)],
                check=True, capture_output=True,
            )
            pieces.append(sil)

    # Build concat list
    list_file = work / "concat.txt"
    with list_file.open("w") as f:
        for p in pieces:
            f.write(f"file '{p}'\n")

    out_wav = work / "narration.wav"
    subprocess.run(
        ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(list_file),
         "-c:a", "pcm_s16le", str(out_wav)],
        check=True, capture_output=True,
    )

    # Re-encode to MP3 for muxer compatibility
    out_mp3 = OUT / "narration_track.mp3"
    subprocess.run(
        ["ffmpeg", "-y", "-i", str(out_wav),
         "-c:a", "libmp3lame", "-b:a", "192k", str(out_mp3)],
        check=True, capture_output=True,
    )

    final = ffprobe_duration(out_mp3)
    print(f"\n✅ {out_mp3}  ({final:.2f}s)")


if __name__ == "__main__":
    main()
