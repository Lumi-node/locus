#!/usr/bin/env python3
"""Generate pitch narration (≤2 min) via Qwen3-TTS-CustomVoice (speaker=ryan).

Inputs:
    demo/narration-pitch.json     (5 segments, p1..p5)

Outputs:
    demo/audio/pitch_p1.wav … pitch_p5.wav
    demo/audio/narration_pitch.mp3   (concatenated, padded to 120s)
"""
from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
import torch

from qwen_tts import Qwen3TTSModel

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "demo" / "narration-pitch.json"
OUT = ROOT / "demo" / "audio"
OUT.mkdir(parents=True, exist_ok=True)

MODEL_PATH = "/media/lumi-node/Storage2/AI-Research-Lab/models/repository/other/Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"


def ffprobe_duration(p: Path) -> float:
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(p)],
        capture_output=True, text=True, check=True,
    )
    return float(r.stdout.strip())


def main() -> None:
    spec = json.loads(SPEC.read_text())
    segs = spec["segments"]
    speaker = spec.get("voice", "ryan")

    print(f"loading {MODEL_PATH}  speaker={speaker}")
    model = Qwen3TTSModel.from_pretrained(
        MODEL_PATH,
        device_map="cuda:0" if torch.cuda.is_available() else "cpu",
        dtype=torch.bfloat16,
    )
    print("loaded.")

    seg_paths = []
    for seg in segs:
        sid = seg["id"]
        text = seg["text"]
        instruct = seg.get("instruct", "")
        print(f"\n── {sid} {seg['title']} ({seg['duration_sec']}s) ──")
        print(f"  instruct: {instruct[:80]}…")
        print(f"  text:     {text[:80]}…")
        wavs, sr = model.generate_custom_voice(
            text=text,
            speaker=speaker,
            instruct=instruct,
            language="english",
            do_sample=True, temperature=0.85, top_p=0.95, top_k=50,
            repetition_penalty=1.05,
        )
        wav = np.asarray(wavs[0]).astype(np.float32)
        out = OUT / f"pitch_{sid}.wav"
        sf.write(str(out), wav, sr)
        dur = wav.shape[-1] / sr
        print(f"  → {out.name}  {dur:.2f}s @ {sr}Hz")
        seg_paths.append((seg, out))

    work = OUT / "_work_pitch"
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
                ["ffmpeg", "-y", "-f", "lavfi", "-i", "anullsrc=r=24000:cl=mono",
                 "-t", f"{pad_sec:.3f}", "-c:a", "pcm_s16le", str(sil)],
                check=True, capture_output=True,
            )
            pieces.append(sil)

    list_file = work / "concat.txt"
    with list_file.open("w") as f:
        for p in pieces:
            f.write(f"file '{p}'\n")

    out_wav = work / "narration.wav"
    subprocess.run(
        ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(list_file),
         "-ar", "24000", "-ac", "1", "-c:a", "pcm_s16le", str(out_wav)],
        check=True, capture_output=True,
    )
    out_mp3 = OUT / "narration_pitch.mp3"
    subprocess.run(
        ["ffmpeg", "-y", "-i", str(out_wav),
         "-c:a", "libmp3lame", "-b:a", "192k", str(out_mp3)],
        check=True, capture_output=True,
    )
    print(f"\n✅ {out_mp3}  ({ffprobe_duration(out_mp3):.2f}s)")


if __name__ == "__main__":
    main()
