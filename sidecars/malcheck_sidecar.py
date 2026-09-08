#!/usr/bin/env python3
"""UNVEIL MalCheck sidecar. schema 2.1 via mau.report_generator. Dynamic skipped. No fake Ghidra ok."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path
from typing import Any

_SURFACE_META = frozenset({"status", "reason", "hashes", "file_name", "isolation", "sample", "error"})
_STATIC_SUCCESS = frozenset({"ok", "completed"})


def _cursor_root() -> Path:
    env = os.environ.get("UNVEIL_CURSOR_ROOT")
    if env:
        return Path(env)
    return Path(__file__).resolve().parents[2]


def _emit(payload: dict[str, Any], code: int = 0) -> int:
    sys.stdout.write(json.dumps(payload, ensure_ascii=False, default=str) + "\n")
    return code


def _fail(reason: str) -> int:
    return _emit({"ok": False, "reason": reason}, 1)


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _short_reason(exc: BaseException, limit: int = 240) -> str:
    if hasattr(exc, "message"):
        text = str(getattr(exc, "message"))
    else:
        text = str(exc)
    text = " ".join(text.split())
    return text[:limit] if text else type(exc).__name__


def _parse_bool_flag(value: str) -> bool:
    return value.strip().lower() in {"1", "true", "yes", "on"}


def _ghidra_image() -> str:
    return os.environ.get("UNVEIL_GHIDRA_IMAGE") or "ghidra-headless:latest"


def _docker_available() -> bool:
    return shutil.which("docker") is not None


def _has_scanner_output(surface: dict[str, Any]) -> bool:
    if surface.get("scanner_results"):
        return True
    if surface.get("yara_matches"):
        return True
    if surface.get("capa_matches"):
        return True
    if surface.get("file_type"):
        return True
    if surface.get("entropy") is not None:
        return True
    extra = set(surface.keys()) - _SURFACE_META
    return bool(extra)


def _finalize_surface(surface: dict[str, Any], sha: str, sample_name: str) -> dict[str, Any]:
    hashes = surface.get("hashes")
    if not isinstance(hashes, dict):
        hashes = {}
        surface["hashes"] = hashes
    if not hashes.get("sha256"):
        hashes["sha256"] = sha
    surface.setdefault("file_name", sample_name)

    status = surface.get("status")
    if status == "partial":
        surface["status"] = "skipped"
        surface.setdefault("reason", "hashes-only surface not allowed")
        return surface

    if status in ("ok", "skipped", "failed", "error"):
        return surface

    if surface.get("error") is True:
        surface["status"] = "failed"
        surface.setdefault("reason", _short_reason(Exception(str(surface.get("message") or "surface error"))))
        return surface

    if _has_scanner_output(surface):
        surface["status"] = "ok"
    else:
        surface["status"] = "skipped"
        surface.setdefault("reason", "no scanner output")
    return surface


def _run_surface(sample: Path) -> dict[str, Any]:
    try:
        from mau.errors import SurfaceError
        from mau.surface_runner import run_surface_analysis
    except Exception as exc:  # noqa: BLE001
        return {"status": "skipped", "reason": f"surface import failed: {type(exc).__name__}"}

    try:
        surface = run_surface_analysis(str(sample), container=None, timeout_sec=600)
    except SurfaceError as exc:
        return {"status": "skipped", "reason": _short_reason(exc)}
    except Exception as exc:  # noqa: BLE001
        return {"status": "skipped", "reason": _short_reason(exc)}

    if isinstance(surface, dict):
        surface.setdefault("isolation", "lab-fallback")
    return surface


def _run_static(sample: Path, surface: dict[str, Any], ghidra_eligible: bool) -> dict[str, Any]:
    try:
        from mau.errors import DockerError, StaticError
        from mau.static_analyzer import is_analyzable_binary, run_static_analysis
    except Exception as exc:  # noqa: BLE001
        return {
            "status": "skipped",
            "reason": f"static import failed: {type(exc).__name__}",
            "engine": "ghidra_headless",
        }

    eligible = ghidra_eligible and is_analyzable_binary(
        str(surface.get("file_type") or ""),
        sample.suffix,
    )
    if not eligible:
        return {"status": "skipped", "reason": "not eligible", "engine": "ghidra_headless"}

    if not _docker_available():
        return {"status": "skipped", "reason": "docker unavailable", "engine": "ghidra_headless"}

    os.environ.setdefault("MAU_GHIDRA_NETWORK_NONE", "1")
    results_dir = Path(tempfile.mkdtemp(prefix="unveil-malcheck-results-"))
    prior_results = os.environ.get("RESULTS_DIR")
    os.environ["RESULTS_DIR"] = str(results_dir)
    try:
        static = run_static_analysis(
            str(sample),
            image=_ghidra_image(),
            timeout_sec=int(os.environ.get("UNVEIL_GHIDRA_TIMEOUT", "600")),
        )
        if static.get("status") in _STATIC_SUCCESS:
            return static
        if static.get("status") == "failed":
            reason = str(static.get("error") or static.get("detail") or "static analysis failed")
            return {
                "status": "failed",
                "reason": reason[:240],
                "engine": "ghidra_headless",
            }
        return static
    except (StaticError, DockerError) as exc:
        return {"status": "skipped", "reason": _short_reason(exc), "engine": "ghidra_headless"}
    except Exception as exc:  # noqa: BLE001
        return {"status": "skipped", "reason": _short_reason(exc), "engine": "ghidra_headless"}
    finally:
        if prior_results is None:
            os.environ.pop("RESULTS_DIR", None)
        else:
            os.environ["RESULTS_DIR"] = prior_results
        shutil.rmtree(results_dir, ignore_errors=True)


def _validate_static_report(ghidra_eligible: bool, report: dict[str, Any]) -> str | None:
    static = report.get("phase3_static") or {}
    status = static.get("status")
    if status in _STATIC_SUCCESS and not ghidra_eligible:
        return "static success without ghidra eligibility"
    return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="UNVEIL MalCheck sidecar")
    parser.add_argument("--sample", required=True)
    parser.add_argument("--ghidra-eligible", default="false")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    ghidra_eligible = _parse_bool_flag(args.ghidra_eligible)

    malcheck = _cursor_root() / "MalCheck"
    if not (malcheck / "mau").is_dir():
        return _fail("MalCheck/mau not found")
    sys.path.insert(0, str(malcheck))
    os.environ["MAU_EXPORT_REPORTS"] = "0"

    try:
        from mau.report_generator import REPORT_SCHEMA_VERSION, generate_report
    except Exception as exc:  # noqa: BLE001
        return _fail(f"mau.report_generator import failed: {type(exc).__name__}: {exc}")

    if REPORT_SCHEMA_VERSION != "2.1":
        return _fail(f"unexpected schema {REPORT_SCHEMA_VERSION}")

    sample = Path(args.sample).resolve()
    if args.self_test and not sample.is_file():
        sample = Path(tempfile.gettempdir()) / "unveil-malcheck-selftest.bin"
        sample.write_bytes(b"MZ" + b"\x00" * 64)

    if not sample.is_file():
        return _fail("sample not found")

    sha = _sha256(sample)
    surface = _finalize_surface(_run_surface(sample), sha, sample.name)
    dynamic = {"status": "skipped", "reason": "UNVEIL default: dynamic analysis disabled"}
    unpack = {"status": "skipped", "reason": "unpack not enabled in UNVEIL sidecar"}
    static = _run_static(sample, surface, ghidra_eligible)

    out_dir = Path(tempfile.mkdtemp(prefix="unveil-malcheck-"))
    try:
        report = generate_report(
            surface,
            dynamic,
            static,
            sample_name=sample.name,
            unpack=unpack,
            html=False,
            out_dir=out_dir,
            executive_summary_llm=False,
        )
    except Exception as exc:  # noqa: BLE001
        return _fail(f"generate_report failed: {type(exc).__name__}: {exc}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    report.pop("_paths", None)
    static_err = _validate_static_report(ghidra_eligible, report)
    if static_err:
        return _fail(static_err)

    payload = {
        "ok": True,
        "sha256": sha,
        "ghidra_eligible": ghidra_eligible,
        "report": report,
        "notice": "Verdict is heuristic. Magika score is not malice. Dynamic skipped.",
    }
    return _emit(payload, 0)


if __name__ == "__main__":
    raise SystemExit(main())
