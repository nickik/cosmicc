#!/usr/bin/env python3
"""Pinned, compile-only C compatibility inventory. Never runs generated C code."""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def classify(returncode, diagnostic):
    if returncode == 0:
        return "compiled"
    if returncode < 0 or "panicked at" in diagnostic:
        return "compiler-crash"
    if any(s in diagnostic for s in ["floating-point", "FP legalization", "float legalization", "fadd.f", "fsub.f", "fmul.f", "fdiv.f", "fcmp.", "SSA value type f32", "SSA value type f64"]):
        return "floating-point-boundary"
    if "file '" in diagnostic and "not found" in diagnostic:
        return "missing-header"
    if "should be implemented in ISLE" in diagnostic or "Unsupported feature" in diagnostic:
        return "backend-unsupported"
    if "invalid program:" in diagnostic or "invalid macro:" in diagnostic or "invalid syntax:" in diagnostic:
        return "source-diagnostic"
    return "other-failure"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="unmodified extracted pinned suite root")
    parser.add_argument("output", type=Path)
    parser.add_argument("--compiler", type=Path, default=Path("target/debug/cosmicc"))
    parser.add_argument("--timeout", type=float, default=5)
    parser.add_argument("--jobs", type=int, choices=range(1, 9), default=4)
    parser.add_argument("--inventory-only", action="store_true")
    parser.add_argument("--profile", choices=["freestanding", "tagged"], default="freestanding")
    args = parser.parse_args()
    if not 0 < args.timeout <= 60:
        parser.error("timeout must be greater than zero and at most 60 seconds")
    manifest = json.loads(Path(__file__).with_name("c-testsuite.lock.json").read_text())
    source, output = args.source.resolve(), args.output.resolve()
    if output == source or source in output.parents:
        parser.error("output must be outside the upstream source tree")
    # Verify every tracked case, tags and expected-output file, not only passes.
    for item in manifest["files"]:
        path = source / item["path"]
        if not path.is_file() or sha256(path) != item["sha256"]:
            parser.error("pinned source mismatch: " + item["path"])
    actual = sorted(p.name for p in (source / "tests/single-exec").glob("*.c"))
    if actual != sorted(case["name"] for case in manifest["cases"]):
        parser.error("test case inventory differs from pinned source")
    if output.exists() and any(output.iterdir()):
        parser.error("output must be a fresh empty directory to preserve the compiler snapshot and logs")
    output.mkdir(parents=True, exist_ok=True)
    compiler = args.compiler.resolve()
    metadata = {"suite_commit": manifest["commit"], "archive_sha256": manifest["archive_sha256"],
                "policy": manifest["policy"], "evidence": "compilation only; no linking/execution",
                "timeout_seconds": args.timeout, "jobs": args.jobs, "profile": args.profile}
    if not args.inventory_only:
        # Other workers may rebuild the live compiler; every case in this run
        # uses one immutable local snapshot, identified by its executable hash.
        snapshot = output / "compiler-snapshot"
        shutil.copy2(compiler, snapshot)
        metadata.update({"compiler_origin": str(compiler), "compiler_sha256": sha256(snapshot)})
        version = subprocess.run([str(snapshot), "--version"], capture_output=True, text=True, timeout=5)
        metadata["compiler_version"] = (version.stdout + version.stderr).strip()
    def probe(case):
        row = dict(case)
        selected = case["selected"] if args.profile == "freestanding" else case["tag_selected"]
        row["selected"] = selected
        if not selected:
            row.update(status="excluded", reason=case["exclusion_reason"])
            return row
        if args.inventory_only:
            row["status"] = "selected-not-run"
            return row
        case_path = source / "tests/single-exec" / case["name"]
        log_path = output / (case["name"] + ".log")
        command = [str(snapshot), str(case_path), "-o", str(output / (case["name"] + ".sia"))]
        started = time.monotonic()
        with log_path.open("wb") as log:
            process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                returncode = process.wait(timeout=args.timeout)
                diagnostic = log_path.read_text(errors="replace")
                status = classify(returncode, diagnostic)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
                returncode, status = None, "timeout"
                diagnostic = log_path.read_text(errors="replace")
        row.update(status=status, returncode=returncode, seconds=round(time.monotonic()-started, 3),
                   diagnostic=diagnostic.splitlines()[:5], command=command,
                   log=log_path.name, source_sha256=sha256(case_path))
        return row
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(probe, manifest["cases"]))
    summary = dict(sorted(Counter(row["status"] for row in results).items()))
    report = {"metadata": metadata, "summary": summary, "results": results}
    (output / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    (output / "summary.tsv").write_text("case\tstatus\ttags\n" + "".join(
        row["name"] + "\t" + row["status"] + "\t" + " ".join(row["tags"]) + "\n" for row in results))
    # Only zero-output, no-argument mains can enter the initial native driver.
    execution = []
    import re
    for row in results:
        if row["status"] == "compiled":
            text = (source / "tests/single-exec" / row["name"]).read_text()
            signatures = re.findall(r"\bmain\s*\(([^)]*)\)", text)
            output_empty = (source / "tests/single-exec" / (row["name"] + ".expected")).stat().st_size == 0
            if output_empty and signatures and all(s.strip() in ["", "void"] for s in signatures):
                execution.append(row["name"] + ".sia")
    (output / "execution-candidates.txt").write_text("\n".join(execution) + "\n")
    print(json.dumps(summary, sort_keys=True))
    print("Compilation inventory complete. This is not an upstream execution-suite pass.")


if __name__ == "__main__":
    main()
