#!/usr/bin/env python3
"""UNVEIL USB-GuardDuty sidecar. Target is read-only; writes go to host temp only."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
from typing import Any

DRIVE_REMOVABLE = 2


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


def _folder_mode_enabled(flag: bool) -> bool:
    return flag or os.environ.get("UNVEIL_USB_FOLDER_MODE") == "1"


def _path_on_removable_drive(root: Path) -> bool:
    if sys.platform != "win32":
        return False
    import ctypes

    resolved = root.resolve()
    drive = resolved.drive
    if not drive:
        return False
    root_spec = f"{drive}\\"
    return ctypes.windll.kernel32.GetDriveTypeW(root_spec) == DRIVE_REMOVABLE


def _letter_short(letter: str) -> str:
    text = letter.rstrip("\\")
    if len(text) >= 2 and text[1] == ":":
        return text[:2]
    if len(text) == 1:
        return f"{text.upper()}:"
    return text


def _list_drives() -> dict[str, Any]:
    ugd = _cursor_root() / "USB-GuardDuty"
    sys.path.insert(0, str(ugd))
    try:
        from backend.drives import list_removable_drives
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "reason": f"drives import failed: {type(exc).__name__}: {exc}"}

    drives: list[dict[str, Any]] = []
    for item in list_removable_drives():
        letter = _letter_short(str(item.get("letter") or ""))
        if not letter:
            continue
        token_hint = hashlib.sha256(letter.encode("utf-8")).hexdigest()[:8]
        drives.append(
            {
                "token_hint": token_hint,
                "letter": letter,
                "bus": "USB",
                "vid": item.get("vid"),
                "pid": item.get("pid"),
                "composite_suspect": bool(item.get("composite_suspect")),
                "capacity_bytes": item.get("size_bytes"),
            }
        )
    return {"ok": True, "drives": drives}


def _relative_inventory(root: Path, files_meta: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for item in files_meta:
        raw = Path(str(item.get("path") or ""))
        try:
            rel = raw.relative_to(root).as_posix()
        except ValueError:
            rel = raw.name
        out.append({"name": rel, "sha256": item.get("sha256"), "size": item.get("size")})
    return out


def _scan_root(root: Path, workers: int, folder_mode: bool) -> dict[str, Any]:
    if not _folder_mode_enabled(folder_mode) and not _path_on_removable_drive(root):
        return {"ok": False, "reason": "not_removable"}

    ugd = _cursor_root() / "USB-GuardDuty"
    sys.path.insert(0, str(ugd))
    try:
        from backend.pipeline import run_scan
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "reason": f"usb pipeline import failed: {type(exc).__name__}: {exc}"}

    worker_count = max(1, min(int(workers), 8))
    try:
        result = run_scan(root, online=False, workers=worker_count)
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "reason": f"usb scan failed: {type(exc).__name__}: {exc}"}

    inventory = _relative_inventory(root, result.get("inventory") or [])
    hashes = sorted(str(item.get("sha256") or "") for item in inventory if item.get("sha256"))
    digest = hashlib.sha256("\n".join(hashes).encode("utf-8")).hexdigest() if hashes else None
    return {
        "ok": True,
        "sha256_root": digest,
        "files_scanned": result.get("files_scanned"),
        "files_total": result.get("files_total"),
        "overall": result.get("overall"),
        "max_score": result.get("max_score"),
        "findings_count": len(result.get("findings") or []),
        "inventory": inventory,
        "workers": worker_count,
        "notice": "USB clean is not a safety verdict. Scan is static and read-only.",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="UNVEIL USB sidecar")
    parser.add_argument("--list-drives", action="store_true")
    parser.add_argument("--root", default="")
    parser.add_argument("--readonly", action="store_true")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--folder-mode", action="store_true")
    parser.add_argument("--online", action="store_true")
    args = parser.parse_args(argv)

    if args.online:
        return _fail("online is opt-in only via explicit coordinator flag (default off)")

    if args.list_drives:
        payload = _list_drives()
        return _emit(payload, 0 if payload.get("ok") else 1)

    if not args.root:
        return _fail("root required unless --list-drives")
    if not args.readonly:
        return _fail("readonly flag required")

    root = Path(args.root).resolve()
    if not root.is_dir():
        return _fail("root is not a directory")

    payload = _scan_root(root, args.workers, args.folder_mode)
    return _emit(payload, 0 if payload.get("ok") else 1)


if __name__ == "__main__":
    raise SystemExit(main())
