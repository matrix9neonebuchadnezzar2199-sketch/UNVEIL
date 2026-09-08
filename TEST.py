#!/usr/bin/env python3
"""UNVEIL verification entry.

    python TEST.py

Builds the worker/ctl, runs cargo tests, isolation diagnose, and unveil-ctl smoke
(import -> detect -> preview -> approve -> apply -> wiki -> report).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

REPO_ROOT = Path(__file__).resolve().parent
Status = Literal["pass", "fail", "skip", "warn"]


def _safe_print(text: str = "") -> None:
    try:
        print(text)
    except UnicodeEncodeError:
        print(text.encode("ascii", "replace").decode("ascii"))


@dataclass
class CheckResult:
    name: str
    status: Status
    detail: str


@dataclass
class Report:
    results: list[CheckResult] = field(default_factory=list)

    def add(self, name: str, status: Status, detail: str) -> None:
        self.results.append(CheckResult(name, status, detail))

    @property
    def failed(self) -> bool:
        return any(r.status == "fail" for r in self.results)

    def print_summary(self) -> None:
        icons = {"pass": "OK", "fail": "FAIL", "skip": "SKIP", "warn": "WARN"}
        _safe_print("\n=== UNVEIL verification summary ===")
        for r in self.results:
            _safe_print(f"  [{icons[r.status]:4}] {r.name}: {r.detail}")
        _safe_print()


def _run(cmd: list[str], timeout: int = 600) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=timeout,
        errors="replace",
    )


def cargo() -> str:
    return "cargo"


def main() -> int:
    report = Report()
    env_ok = True

    wiki_count = len(list((REPO_ROOT / "knowledge" / "pack-1.0.0" / "articles").glob("*.md")))
    report.add(
        "P04 wiki articles",
        "pass" if wiki_count >= 12 else "fail",
        f"{wiki_count} markdown articles",
    )

    docs = [REPO_ROOT / "docs" / f"{n}.md" for n in [
        "00-product-requirements",
        "01-phase-plan",
        "02-architecture",
        "03-analysis-pipeline",
        "04-data-contracts",
        "05-wiki-knowledge",
        "06-ui-ux",
        "07-security",
        "08-quality-validation",
        "09-decisions-operations",
    ]]
    missing = [p.name for p in docs if not p.is_file()]
    report.add("P00 design docs", "pass" if not missing else "fail", "ok" if not missing else str(missing))

    build = _run(
        [cargo(), "build", "-p", "unveil-analysis-worker", "-p", "unveil-coordinator", "--bins"],
        timeout=900,
    )
    if build.returncode != 0:
        report.add("build worker+ctl", "fail", (build.stderr or build.stdout)[-1200:])
        report.print_summary()
        return 1
    report.add("build worker+ctl", "pass", "debug bins")

    tests = _run(
        [cargo(), "test", "--workspace", "--exclude", "unveil", "--offline"]
        if False
        else [cargo(), "test", "--workspace", "--exclude", "unveil"],
        timeout=900,
    )
    if tests.returncode != 0:
        report.add("cargo test", "fail", (tests.stderr or tests.stdout)[-2000:])
        report.print_summary()
        return 1
    report.add("cargo test (exclude tauri)", "pass", "workspace")

    ctl = REPO_ROOT / "target" / "debug" / ("unveil-ctl.exe" if os.name == "nt" else "unveil-ctl")
    worker = REPO_ROOT / "target" / "debug" / (
        "unveil-worker.exe" if os.name == "nt" else "unveil-worker"
    )
    env = os.environ.copy()
    env["UNVEIL_WORKER"] = str(worker)
    env["UNVEIL_KNOWLEDGE"] = str(REPO_ROOT / "knowledge" / "pack-1.0.0")

    refuse = subprocess.run(
        [str(worker)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=20,
        errors="replace",
    )
    if refuse.returncode == 1:
        report.add("ST worker refuse without --isolated", "pass", f"exit={refuse.returncode}")
    else:
        report.add("ST worker refuse without --isolated", "fail", f"exit={refuse.returncode}")

    diag = subprocess.run(
        [str(ctl), "diagnose"],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=60,
        errors="replace",
    )
    try:
        payload = json.loads(diag.stdout or "{}")
    except json.JSONDecodeError:
        payload = {}
    if diag.returncode == 0 and payload.get("available") is True:
        report.add("G01 isolation diagnose", "pass", "AppContainer+Job self-test")
    else:
        report.add(
            "G01 isolation diagnose",
            "fail",
            (diag.stdout or diag.stderr)[-1500:] or str(payload),
        )
        env_ok = False

    if env_ok:
        smoke = subprocess.run(
            [str(ctl), "smoke"],
            cwd=REPO_ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=180,
            errors="replace",
        )
        if smoke.returncode == 0:
            report.add("AT smoke (01-06)", "pass", (smoke.stdout or "").strip()[-400:])
        else:
            report.add(
                "AT smoke (01-06)",
                "fail",
                (smoke.stderr or smoke.stdout)[-2000:],
            )
    else:
        report.add("AT smoke (01-06)", "fail", "skipped because isolation is unavailable")

    tsc_bin = REPO_ROOT / "apps" / "desktop" / "node_modules" / ".bin" / (
        "tsc.cmd" if os.name == "nt" else "tsc"
    )
    tsc_cmd = (
        [str(tsc_bin), "--noEmit", "-p", str(REPO_ROOT / "apps" / "desktop" / "tsconfig.json")]
        if tsc_bin.is_file()
        else ["npx.cmd" if os.name == "nt" else "npx", "--yes", "tsc", "--noEmit", "-p", "apps/desktop/tsconfig.json"]
    )
    try:
        tsc = _run(tsc_cmd, timeout=120)
        if tsc.returncode == 0:
            report.add("P05 tsc", "pass", "apps/desktop")
        else:
            report.add("P05 tsc", "fail", (tsc.stderr or tsc.stdout)[-800:])
    except FileNotFoundError:
        report.add("P05 tsc", "fail", "tsc binary not found")

    report.add(
        "P07 unpackers",
        "skip",
        "separate roadmap: PE/ELF internal unpack not in MVP",
    )

    env["UNVEIL_ROOT"] = str(REPO_ROOT)
    _run_upgrade_probes(report, ctl, env)

    report.print_summary()
    return 1 if report.failed else 0


def _ctl_json(ctl: Path, env: dict[str, str], *args: str, timeout: int = 180) -> tuple[int, dict]:
    proc = subprocess.run(
        [str(ctl), *args],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=timeout,
        errors="replace",
    )
    text = (proc.stdout or "").strip()
    try:
        payload = json.loads(text) if text else {}
    except json.JSONDecodeError:
        payload = {"_raw": text, "_err": (proc.stderr or "")[-1500:]}
    if not payload and proc.stderr:
        payload = {"_err": proc.stderr[-1500:]}
    return proc.returncode, payload if isinstance(payload, dict) else {"_value": payload}


def _run_upgrade_probes(report: Report, ctl: Path, env: dict[str, str]) -> None:
    import hashlib
    import tempfile
    import time

    docs_u = [REPO_ROOT / "docs" / f"{n}.md" for n in [
        "10-unified-workbench",
        "11-module-architecture",
        "12-upgrade-phase-plan",
        "13-malware-module",
        "14-usb-module",
        "15-magika-absorption",
        "16-depth-phase-plan",
        "17-depth-implementation-spec",
    ]]
    missing = [p.name for p in docs_u if not p.is_file()]
    nine = (REPO_ROOT / "docs" / "09-decisions-operations.md").read_text(encoding="utf-8")
    oq_ok = "OQ-U01" in nine and "ADR-U001" in nine
    report.add(
        "U00 design docs",
        "pass" if not missing and oq_ok else "fail",
        "ok" if not missing and oq_ok else f"missing={missing} oq={oq_ok}",
    )

    code, modules = _ctl_json(ctl, env, "modules")
    work = [
        m for m in (modules.get("modules") or [])
        if isinstance(m, dict) and m.get("id") != "dashboard"
    ]
    ids = [m.get("id") for m in work]
    planned_disabled = all(
        m.get("disabled") is True
        for m in work
        if m.get("status") in ("planned", "disabled")
    )
    u01_ok = (
        code == 0
        and ids == ["malware", "deobfuscation", "usb"]
        and planned_disabled
    )
    report.add("U01 manifest renders", "pass" if u01_ok else "fail", str(ids))

    iso_code, iso = _ctl_json(ctl, env, "diagnose")
    iso_ok = iso.get("available") is True
    d_code, deobf = _ctl_json(ctl, env, "diagnose-module", "deobfuscation")
    mal_code, malware = _ctl_json(ctl, env, "diagnose-module", "malware")
    usb_code, usb = _ctl_json(ctl, env, "diagnose-module", "usb")
    deobf_matches = deobf.get("available") is iso_ok
    sidecar_mal = any(
        c.get("id") == "C_sidecar_malcheck" and c.get("available") is False
        for c in (malware.get("checks") or [])
        if isinstance(c, dict)
    )
    fake_success = malware.get("available") is True and sidecar_mal
    u02_ok = deobf_matches and not fake_success and (iso_ok == (iso_code == 0))
    report.add(
        "U02 isolation still fail-closed",
        "pass" if u02_ok else "fail",
        f"iso={iso_ok} deobf={deobf.get('available')} malware={malware.get('available')} usb={usb.get('available')}",
    )

    magika_script = REPO_ROOT / "sidecars" / "magika_probe.py"
    magika_proc = subprocess.run(
        [sys.executable, str(magika_script), "--self-test"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=60,
        errors="replace",
    )
    try:
        magika = json.loads((magika_proc.stdout or "").strip() or "{}")
    except json.JSONDecodeError:
        magika = {}
    magika_status = magika.get("status")
    u03_ok = magika_status in ("ok", "unavailable")
    report.add(
        "U03 magika model loads",
        "pass" if u03_ok else "fail",
        f"status={magika_status}",
    )

    with tempfile.TemporaryDirectory(prefix="unveil-u04-") as tmp:
        tdir = Path(tmp)
        pe = b"MZ" + (b"\x00" * 64)
        pe_path = tdir / "invoice.txt"
        pe_path.write_bytes(pe)
        _, probe = _ctl_json(ctl, env, "probe", "--path", str(pe_path), "--name", "invoice.txt")
        flags = probe.get("mismatch_flags") or []
        u04_ok = "extension_vs_magic" in flags
        report.add("U04 mismatch fixture", "pass" if u04_ok else "fail", str(flags))

        js_path = tdir / "app.js"
        js_path.write_text("function hello(){return 1;}\n", encoding="utf-8")
        _, js_probe = _ctl_json(ctl, env, "probe", "--path", str(js_path), "--name", "app.js")
        u05_ok = js_probe.get("ghidra_eligible") is False
        report.add(
            "U05 routing",
            "pass" if u05_ok else "fail",
            f"ghidra_eligible={js_probe.get('ghidra_eligible')}",
        )

        sample = tdir / "sample.bin"
        sample.write_bytes(pe)
        mal_rc, mal_job = _ctl_json(ctl, env, "malcheck", "--sample", str(sample), timeout=180)
        schema = (
            ((mal_job.get("payload") or {}).get("report") or {}).get("meta") or {}
        ).get("schema_version")
        dynamic = ((mal_job.get("payload") or {}).get("report") or {}).get("phase2_dynamic") or {}
        u06_ok = (
            mal_rc == 0
            and schema == "2.1"
            and dynamic.get("status") == "skipped"
        )
        report.add(
            "U06 malcheck schema 2.1",
            "pass" if u06_ok else "fail",
            f"rc={mal_rc} schema={schema} dynamic={dynamic.get('status')} err={mal_job.get('_err') or mal_job.get('message_key')}",
        )

        usb_root = tdir / "usbroot"
        usb_root.mkdir()
        target = usb_root / "keep.txt"
        target.write_text("unveil-usb-readonly\n", encoding="utf-8")
        env["UNVEIL_USB_FOLDER_MODE"] = "1"
        before = {
            str(p.relative_to(usb_root)): (p.stat().st_mtime_ns, p.stat().st_size)
            for p in usb_root.rglob("*")
            if p.is_file()
        }
        time.sleep(0.05)
        usb_rc, usb_job = _ctl_json(ctl, env, "usb-scan", "--root", str(usb_root), timeout=180)
        after_names = {str(p.relative_to(usb_root)) for p in usb_root.rglob("*") if p.is_file()}
        after = {
            str(p.relative_to(usb_root)): (p.stat().st_mtime_ns, p.stat().st_size)
            for p in usb_root.rglob("*")
            if p.is_file()
        }
        u07_ok = usb_rc == 0 and before == after and after_names == set(before)
        report.add(
            "U07 usb readonly",
            "pass" if u07_ok else "fail",
            f"rc={usb_rc} wrote={after_names - set(before)} err={usb_job.get('_err') or usb_job.get('message_key')}",
        )

        hand_root = tdir / "handoff"
        hand_root.mkdir()
        hand_file = hand_root / "sample.bin"
        hand_file.write_bytes(pe)
        expect = hashlib.sha256(pe).hexdigest()
        hand_rc, hand_job = _ctl_json(
            ctl,
            env,
            "handoff",
            "--root",
            str(hand_root),
            "--sample",
            str(hand_file),
            timeout=180,
        )
        got = hand_job.get("sha256")
        u08_ok = hand_rc == 0 and got == expect
        report.add(
            "U08 handoff",
            "pass" if u08_ok else "fail",
            f"rc={hand_rc} sha={got} expect={expect[:12]}...",
        )

    models_root = REPO_ROOT / "third_party" / "magika" / "models"
    bundled_model = any(
        (p / "model.onnx").is_file()
        for p in models_root.iterdir()
        if p.is_dir()
    ) if models_root.is_dir() else False
    magika_self = subprocess.run(
        [sys.executable, str(magika_script), "--self-test"],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
        timeout=60,
        errors="replace",
    )
    try:
        magika_self_payload = json.loads((magika_self.stdout or "").strip() or "{}")
    except json.JSONDecodeError:
        magika_self_payload = {}
    u11_self_ok = magika_self_payload.get("status") == "ok"
    js_path_u11 = REPO_ROOT / "sidecars" / "_u11_probe.js"
    js_path_u11.write_text("function hello(){return 1;}\n", encoding="utf-8")
    u11_probe: dict = {}
    try:
        _, u11_probe = _ctl_json(
            ctl,
            env,
            "probe",
            "--path",
            str(js_path_u11),
            "--name",
            "app.js",
        )
        u11_ctl_ok = (u11_probe.get("magika") or {}).get("status") == "ok"
        u11_route_ok = u11_probe.get("ghidra_eligible") is False
    finally:
        js_path_u11.unlink(missing_ok=True)
    u11_ok = bundled_model and u11_self_ok and u11_ctl_ok and u11_route_ok
    report.add(
        "U11 magika bundled",
        "pass" if u11_ok else "fail",
        f"model={bundled_model} self={magika_self_payload.get('status')} "
        f"ctl={(u11_probe.get('magika') or {}).get('status') if bundled_model else 'n/a'} "
        f"ghidra={u11_probe.get('ghidra_eligible') if bundled_model else 'n/a'}",
    )

    refuse_env = {k: v for k, v in env.items() if k != "UNVEIL_USB_FOLDER_MODE"}
    with tempfile.TemporaryDirectory(prefix="unveil-u12-refuse-") as refuse_tmp:
        refuse_root = Path(refuse_tmp)
        refuse_root.mkdir(exist_ok=True)
        (refuse_root / "probe.txt").write_text("u12\n", encoding="utf-8")
        refuse_rc, refuse_job = _ctl_json(
            ctl,
            refuse_env,
            "usb-scan",
            "--root",
            str(refuse_root),
            timeout=180,
        )
    u12_refuse_ok = refuse_rc != 0 or refuse_job.get("ok") is False
    drives_rc, drives_payload = _ctl_json(ctl, env, "usb-drives")
    u12_drives_ok = drives_rc == 0 and isinstance(drives_payload.get("drives"), list)
    with tempfile.TemporaryDirectory(prefix="unveil-u12-workers-") as workers_tmp:
        workers_root = Path(workers_tmp)
        (workers_root / "a.txt").write_text("workers\n", encoding="utf-8")
        workers_rc, _workers_job = _ctl_json(
            ctl,
            env,
            "usb-scan",
            "--root",
            str(workers_root),
            "--folder-mode",
            "--workers",
            "4",
            timeout=180,
        )
    u12_workers_ok = workers_rc == 0
    u12_ok = u12_refuse_ok and u12_drives_ok and u12_workers_ok and u07_ok
    report.add(
        "U12 removable refuse",
        "pass" if u12_ok else "fail",
        f"refuse_rc={refuse_rc} drives={u12_drives_ok} workers={u12_workers_ok} u07={u07_ok}",
    )

    u13_surface = ((mal_job.get("payload") or {}).get("report") or {}).get("phase1_surface") or {}
    u13_reason = str(u13_surface.get("reason") or "")
    u13_status = u13_surface.get("status")
    u13_allowed = u13_status in ("ok", "skipped", "failed", "error")
    u13_no_stub = "surface-minimal" not in u13_reason
    u13_scanner = True
    if u13_status == "ok":
        meta_keys = {"status", "reason", "hashes", "file_name", "isolation"}
        scanner_keys = set(u13_surface.keys()) - meta_keys
        u13_scanner = bool(scanner_keys)
    u13_static = ((mal_job.get("payload") or {}).get("report") or {}).get("phase3_static") or {}
    u13_ok = (
        u13_no_stub
        and u13_allowed
        and u13_scanner
        and u06_ok
    )
    report.add(
        "U13 surface honest",
        "pass" if u13_ok else "fail",
        f"status={u13_status} stub={not u13_no_stub} scanner={u13_scanner} static={u13_static.get('status')}",
    )

    static_success = frozenset({"ok", "completed"})
    js_path_u14 = REPO_ROOT / "sidecars" / "_u14_app.js"
    js_path_u14.write_text("function u14(){return 0;}\n", encoding="utf-8")
    try:
        u14_js_rc, u14_js_job = _ctl_json(
            ctl, env, "malcheck", "--sample", str(js_path_u14), timeout=600
        )
    finally:
        js_path_u14.unlink(missing_ok=True)
    u14_js_payload = u14_js_job.get("payload") or {}
    u14_js_static = (u14_js_payload.get("report") or {}).get("phase3_static") or {}
    u14_js_static_status = u14_js_static.get("status")
    u14_js_ok = (
        u14_js_rc == 0
        and u14_js_payload.get("ghidra_eligible") is False
        and u14_js_static_status not in static_success
    )
    u14_pe_payload = mal_job.get("payload") or {}
    u14_pe_eligible = u14_pe_payload.get("ghidra_eligible")
    u14_pe_static = u13_static
    u14_pe_static_status = u14_pe_static.get("status")
    u14_no_forgery = True
    if u14_pe_payload.get("ghidra_eligible") is False and u14_pe_static_status in static_success:
        u14_no_forgery = False
    if u14_js_payload.get("ghidra_eligible") is False and u14_js_static_status in static_success:
        u14_no_forgery = False
    u14_pe_ok = u14_pe_static_status not in static_success or u14_pe_eligible is True
    u14_ok = u14_js_ok and u14_no_forgery and u14_pe_ok and u05_ok and u06_ok
    report.add(
        "U14 ghidra skipped-or-real",
        "pass" if u14_ok else "fail",
        f"js_static={u14_js_static_status} pe_static={u14_pe_static_status} pe_eligible={u14_pe_eligible} forgery={u14_no_forgery}",
    )

    import re

    users_pat = re.compile(r"[A-Za-z]:\\Users\\")
    json_blobs = [
        json.dumps(mal_job, default=str),
        json.dumps(usb_job, default=str),
        json.dumps(u14_js_job, default=str),
    ]
    u15_no_user_paths = all(not users_pat.search(text) for text in json_blobs)
    inv_names = [
        str(row.get("name") or "")
        for row in ((usb_job.get("payload") or {}).get("inventory") or [])
    ]
    u15_relative = all(name and not Path(name).is_absolute() for name in inv_names) if inv_names else True
    with tempfile.TemporaryDirectory(prefix="unveil-u15-") as u15_tmp:
        u15_root = Path(u15_tmp)
        u15_file = u15_root / "u15-handoff.bin"
        u15_file.write_bytes(pe)
        u15_hf_rc, u15_hf_job = _ctl_json(
            ctl,
            env,
            "handoff-file",
            "--root",
            str(u15_root),
            "--name",
            "u15-handoff.bin",
            "--folder-mode",
            timeout=600,
        )
    u15_handoff_ok = u15_hf_rc == 0 and bool(u15_hf_job.get("input_artifact_id"))
    u15_ok = u15_no_user_paths and u15_relative and u15_handoff_ok and u08_ok
    report.add(
        "U15 handoff ui",
        "pass" if u15_ok else "fail",
        f"paths={u15_no_user_paths} relative={u15_relative} artifact={u15_hf_job.get('input_artifact_id')} u08={u08_ok}",
    )

    lockfile = REPO_ROOT / "sidecars" / "requirements.lock"
    doc08 = REPO_ROOT / "docs" / "08-quality-validation.md"
    u16_lock_ok = lockfile.is_file()
    u16_doc_ok = False
    if doc08.is_file():
        u16_doc_ok = "AT-U11" in doc08.read_text(encoding="utf-8")
    u16_ok = u16_lock_ok and u16_doc_ok
    report.add(
        "U16 sbom",
        "pass" if u16_ok else "fail",
        f"lock={u16_lock_ok} doc08={u16_doc_ok}",
    )

    _ = d_code, mal_code, usb_code


if __name__ == "__main__":
    sys.exit(main())
