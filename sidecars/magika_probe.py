#!/usr/bin/env python3
"""UNVEIL Magika content-type probe. Missing model => status=unavailable (no libmagic fallback)."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any


def _repo_root() -> Path:
    if root := os.environ.get("UNVEIL_ROOT"):
        return Path(root)
    return Path(__file__).resolve().parent.parent


def _ensure_vendor_path() -> None:
    vendor = _repo_root() / "sidecars" / "vendor"
    if vendor.is_dir():
        text = str(vendor)
        if text not in sys.path:
            sys.path.insert(0, text)


def _resolve_model_dir(repo: Path) -> Path | None:
    if env := os.environ.get("UNVEIL_MAGIKA_MODEL_DIR"):
        candidate = Path(env)
        if _model_files_ok(candidate):
            return candidate
        return None
    models_root = repo / "third_party" / "magika" / "models"
    if not models_root.is_dir():
        return None
    children = sorted(p for p in models_root.iterdir() if p.is_dir())
    if len(children) != 1:
        return None
    candidate = children[0]
    return candidate if _model_files_ok(candidate) else None


def _model_files_ok(model_dir: Path) -> bool:
    return (model_dir / "model.onnx").is_file() and (model_dir / "config.min.json").is_file()


def _emit(payload: dict[str, Any], code: int = 0) -> int:
    sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
    return code


def _unavailable(reason: str) -> dict[str, Any]:
    return {
        "status": "unavailable",
        "dl_label": None,
        "output_label": None,
        "score": None,
        "prediction_mode": "high-confidence",
        "mime_type": None,
        "is_text": None,
        "reason": reason,
    }


def _label_of(obj: Any) -> str | None:
    if obj is None:
        return None
    for attr in ("label", "ct_label", "output_label"):
        value = getattr(obj, attr, None)
        if value:
            return str(value)
    if isinstance(obj, str):
        return obj
    return None


def _prediction_mode_name(mode: Any | None = None) -> str:
    raw = str(mode) if mode is not None else "high-confidence"
    if raw in {"high_confidence", "PredictionMode.HIGH_CONFIDENCE"}:
        return "high-confidence"
    if raw in {"medium_confidence", "PredictionMode.MEDIUM_CONFIDENCE"}:
        return "medium-confidence"
    if raw in {"best_guess", "PredictionMode.BEST_GUESS"}:
        return "best-guess"
    return raw.replace("_", "-")


def _identify_with_magika(data: bytes | None, path: Path | None) -> dict[str, Any]:
    _ensure_vendor_path()
    model_dir = _resolve_model_dir(_repo_root())
    if model_dir is None:
        return _unavailable("bundled model directory missing")

    try:
        from magika import Magika
        from magika.types.prediction_mode import PredictionMode
    except Exception as exc:  # noqa: BLE001 — probe must never crash the broker
        return _unavailable(f"magika import failed: {type(exc).__name__}")

    try:
        magika = Magika(model_dir=model_dir, prediction_mode=PredictionMode.HIGH_CONFIDENCE)
        if path is not None:
            result = magika.identify_path(path)
        else:
            result = magika.identify_bytes(data or b"")
    except Exception as exc:  # noqa: BLE001
        return _unavailable(f"magika identify failed: {type(exc).__name__}")

    output = getattr(result, "output", result)
    dl = getattr(result, "dl", None)
    score = getattr(result, "score", None)
    if score is None:
        score = getattr(output, "score", None)
    mime = getattr(output, "mime_type", None) or getattr(result, "mime_type", None)
    is_text = getattr(output, "is_text", None)
    if is_text is None:
        is_text = getattr(result, "is_text", None)
    label = _label_of(output) or _label_of(result)
    if not label:
        return _unavailable("magika returned empty label")
    return {
        "status": "ok",
        "dl_label": _label_of(dl) or label,
        "output_label": label,
        "score": float(score) if score is not None else None,
        "prediction_mode": _prediction_mode_name(PredictionMode.HIGH_CONFIDENCE),
        "mime_type": str(mime) if mime else None,
        "is_text": bool(is_text) if is_text is not None else None,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="UNVEIL Magika probe")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--path", default="")
    args = parser.parse_args(argv)
    if args.self_test:
        payload = _identify_with_magika(b"function hello(){return 1;}\n", None)
        return _emit(payload, 0)
    if not args.path:
        return _emit(_unavailable("path required"), 0)
    path = Path(args.path)
    if not path.is_file():
        return _emit(_unavailable("file not found"), 0)
    return _emit(_identify_with_magika(None, path), 0)


if __name__ == "__main__":
    raise SystemExit(main())
