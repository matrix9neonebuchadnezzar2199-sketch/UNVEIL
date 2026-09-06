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

    report.print_summary()
    return 1 if report.failed else 0


if __name__ == "__main__":
    sys.exit(main())
