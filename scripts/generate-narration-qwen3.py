#!/usr/bin/env python3
"""High-energy founder pitch narration via Qwen3-TTS-VoiceDesign (1.7B, 4.3GB).

The VoiceDesign variant of Qwen3-TTS takes a natural-language `instruct` field
that controls timbre, emotion, and prosody. We use it to ask for an
*enthusiastic* delivery — the antidote to CSM-1B's flat podcast tone.

Outputs:
    demo/audio/qwen3_s1.wav … qwen3_s6.wav   (sample rate from model)
    demo/audio/narration_track.mp3           (concatenated, padded to 180s)
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
SPEC = ROOT / "demo" / "narration.json"
OUT = ROOT / "demo" / "audio"
OUT.mkdir(parents=True, exist_ok=True)

MODEL_PATH = "/media/lumi-node/Storage2/AI-Research-Lab/models/repository/other/Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign"

# Per-section voice instructions. Same speaker character throughout, but tuned
# for emotional fit: building tension early, peaking at proof, calm-confident close.
VOICE_INSTRUCTIONS = {
    "s1": "An energetic young American male tech founder, mid-30s, opening a Solana hackathon pitch. "
          "Confident, slightly urgent, leaning into the problem. Clear articulation, varied intonation, "
          "no monotony. American English, broadcast quality.",
    "s2": "Same male founder, hitting the key insight. Tone shifts to thoughtful and emphatic — like "
          "he's delivering the big idea. Slow down on key phrases like 'coordinate problem' and "
          "'cryptographically committable'. American English, expressive.",
    "s3": "Same male founder, talking about his year of research. Confident, slightly proud, "
          "matter-of-fact when reciting benchmark numbers. Brisker pace. American English.",
    "s4": "Same male founder, technical explanation mode. Crisp, articulate, slightly faster. "
          "Each clause lands cleanly. American English, natural cadence.",
    "s5": "Same male founder, demoing on stage. ENERGETIC and proud, peaks of excitement on "
          "'live right now' and 'real decoded'. Like he's showing off something he just shipped. "
          "American English, high-energy founder voice.",
    "s6": "Same male founder, closing the pitch. Confident, conviction-laden, slightly slower for "
          "the final ask. The 'looking for accelerator placement' line should sound deliberate "
          "and ambitious. American English, conviction over volume.",
}


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

    print(f"loading {MODEL_PATH}")
    print(f"  cuda available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        free, total = torch.cuda.mem_get_info()
        print(f"  vram free: {free / 1024**3:.1f} GB / {total / 1024**3:.1f} GB")

    model = Qwen3TTSModel.from_pretrained(
        MODEL_PATH,
        device_map="cuda:0" if torch.cuda.is_available() else "cpu",
        dtype=torch.bfloat16,
    )
    print("loaded.")

    seg_paths: list[tuple[dict, Path]] = []
    for seg in segs:
        sid = seg["id"]
        instruct = VOICE_INSTRUCTIONS[sid]
        text = seg["text"]
        print(f"\n── {sid} {seg['title']} ({seg['duration_sec']}s) ──")
        print(f"  instruct: {instruct[:90]}…")
        print(f"  text:     {text[:90]}…")
        wavs, sr = model.generate_voice_design(
            text=text,
            instruct=instruct,
            language="English",
            do_sample=True,
            temperature=0.85,
            top_p=0.95,
            top_k=50,
            repetition_penalty=1.05,
        )
        wav = np.asarray(wavs[0]).astype(np.float32)
        out = OUT / f"qwen3_{sid}.wav"
        sf.write(str(out), wav, sr)
        dur = wav.shape[-1] / sr
        print(f"  → {out.name}  {dur:.2f}s @ {sr}Hz")
        seg_paths.append((seg, out))

    # Concat with padding to align with timeline.
    work = OUT / "_work_qwen3"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir()

    pieces: list[Path] = []
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

    # Use ffmpeg concat-protocol; sample-rate-mismatch tolerated via decoder.
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

    out_mp3 = OUT / "narration_track.mp3"
    subprocess.run(
        ["ffmpeg", "-y", "-i", str(out_wav),
         "-c:a", "libmp3lame", "-b:a", "192k", str(out_mp3)],
        check=True, capture_output=True,
    )
    print(f"\n✅ {out_mp3}  ({ffprobe_duration(out_mp3):.2f}s)")


if __name__ == "__main__":
    main()
