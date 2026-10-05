#!/usr/bin/env python3
"""Run a deterministic Csmith checksum corpus on GCC and SIA/LightingMachine."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import time

PIN = "0cdc710315cfee9035e22ef4363ca479270d1934"
SOFTFLOAT = [
    "context.c", "binary32-add.c", "binary32-convert.c", "binary64.c",
    "math-bits.c", "math.c",
]


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run(command, log, timeout, cwd=None):
    started = time.monotonic()
    process = subprocess.Popen(
        [str(value) for value in command], cwd=cwd, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True, start_new_session=True,
    )
    try:
        output, _ = process.communicate(timeout=timeout)
        Path(log).write_text(output)
        return process.returncode, time.monotonic() - started, output
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        Path(log).write_text(output + "\nTIMEOUT\n")
        return 124, time.monotonic() - started, output + "\nTIMEOUT\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csmith_source", type=Path, help="clean Csmith source checkout at the pinned revision")
    parser.add_argument("output", type=Path, help="new output directory for seed sources and logs")
    parser.add_argument("--seeds", type=int, default=32, help="run seeds 1..N (default 32)")
    parser.add_argument("--compiler", type=Path, default=Path("target/debug/cosmicc"))
    parser.add_argument("--runner", type=Path, default=Path("tools/l21-execution/target/debug/multi-object-native"))
    parser.add_argument("--timeout", type=int, default=10, help="seconds per compile or execution subprocess")
    parser.add_argument("--runtime-timeout", type=int, default=120, help="seconds per runtime support compile")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    source_root = args.csmith_source.resolve()
    output = args.output.resolve()
    if args.seeds <= 0 or args.timeout <= 0 or args.runtime_timeout <= 0:
        parser.error("--seeds, --timeout and --runtime-timeout must be positive")
    if output.exists():
        parser.error(f"output directory already exists: {output}")
    generator = source_root / "install/bin/csmith"
    include = source_root / "install/include"
    if not generator.exists():
        generator = source_root / "build/src/csmith"
    if not include.is_dir():
        include = source_root / "include"
    if not generator.is_file() or not include.is_dir():
        parser.error("expected built csmith and include/ directory under the supplied checkout")
    revision = subprocess.check_output(["git", "-C", str(source_root), "rev-parse", "HEAD"], text=True).strip()
    if revision != PIN:
        parser.error(f"Csmith revision is {revision}; expected {PIN}")
    if subprocess.check_output(
        ["git", "-C", str(source_root), "status", "--porcelain", "--untracked-files=no"], text=True
    ).strip():
        parser.error("Csmith tracked source checkout has modifications")
    compiler = args.compiler.resolve()
    runner = args.runner.resolve()
    if not compiler.is_file() or not runner.is_file():
        parser.error("build cosmicc and multi-object-native before running this suite")
    gcc = shutil.which("gcc")
    if not gcc:
        parser.error("GCC reference compiler was not found")

    output.mkdir(parents=True)
    cases = output / "cases"
    cases.mkdir()
    runtime_dir = output / "runtime"
    runtime_dir.mkdir()
    runtime_sources = [root / "runtime/softfloat" / name for name in SOFTFLOAT]
    runtime_objects = []
    for source in runtime_sources:
        obj = runtime_dir / f"{source.stem}.sia"
        code, _, _ = run(
            [compiler, "-I", root / "runtime/softfloat", "-o", obj, source],
            runtime_dir / f"{source.stem}.compile.log", args.runtime_timeout, root,
        )
        if code:
            raise SystemExit(f"failed to compile runtime source {source}; see {obj.stem}.compile.log")
        runtime_objects.append(obj)
    support = runtime_dir / "support.c"
    support.write_text(
        "/* Csmith minimal mode never prints; fail closed if it unexpectedly does. */\n"
        "int printf(const char *format, ...) { (void)format; for (;;) {} return 0; }\n"
    )
    support_hash = sha256(support)
    support_obj = runtime_dir / "support.sia"
    code, _, _ = run([compiler, "-o", support_obj, support], runtime_dir / "support.compile.log", args.runtime_timeout, root)
    if code:
        raise SystemExit("failed to compile the simulator-only no-output Csmith support shim")
    runtime_objects.append(support_obj)

    report = {
        "status": "running", "started_at_utc": datetime.now(timezone.utc).isoformat(),
        "suite": "Csmith defined-behavior random C differential corpus",
        "suite_revision": revision,
        "suite_url": f"https://github.com/csmith-project/csmith/commit/{revision}",
        "generator_sha256": sha256(generator), "csmith_header_sha256": sha256(include / "csmith.h"),
        "generator_options": ["--no-argc", "--no-float", "--concise", "--max-block-depth", "2", "--max-block-size", "3", "--max-expr-complexity", "4", "--max-funcs", "3"],
        "seed_set": list(range(1, args.seeds + 1)),
        "per_process_timeout_seconds": args.timeout,
        "simulator_instruction_budget": 30000000,
        "reference": "GCC -std=c11 -O0 with CSMITH_MINIMAL checksum output",
        "target": "Cosmic C SIA32 executed on LightingMachine; checksum is checked by main return",
        "cosmicc_sha256": sha256(compiler), "simulator_runner_sha256": sha256(runner),
        "runner_script_sha256": sha256(Path(__file__).resolve()),
        "gcc_version": subprocess.check_output([gcc, "--version"], text=True).splitlines()[0],
        "gcc_sha256": sha256(Path(gcc).resolve()),
        "runtime_source_sha256": {str(path.relative_to(root)): sha256(path) for path in runtime_sources},
        "simulator_support_source_sha256": support_hash,
        "results": [],
    }
    summary = output / "summary.json"

    def save():
        report["updated_at_utc"] = datetime.now(timezone.utc).isoformat()
        summary.write_text(json.dumps(report, indent=2) + "\n")

    save()
    for seed in report["seed_set"]:
        case = cases / f"seed-{seed:06d}"
        case.mkdir()
        source = case / "generated.c"
        code, elapsed, _ = run(
            [generator, "--seed", seed, "--no-argc", "--no-float", "--concise", "--max-block-depth", "2", "--max-block-size", "3", "--max-expr-complexity", "4", "--max-funcs", "3", "--output", source],
            case / "generate.log", args.timeout, source_root,
        )
        row = {"seed": seed, "status": None}
        if code or not source.exists():
            row.update(status="generator_failed", generator_exit=code)
            report["results"].append(row)
            save()
            continue
        row["source_sha256"] = sha256(source)
        native = case / "gcc-reference"
        gcc_cmd = [gcc, "-std=c11", "-O0", "-w", "-DCSMITH_MINIMAL", "-I", include, source, "-o", native]
        code, elapsed, _ = run(gcc_cmd, case / "gcc-compile.log", args.timeout, case)
        row["gcc_compile_seconds"] = round(elapsed, 6)
        if code:
            row.update(status="gcc_compile_failed", gcc_compile_exit=code)
            report["results"].append(row)
            save()
            continue
        code, elapsed, output_text = run([native], case / "gcc-reference.log", args.timeout, case)
        row["gcc_run_seconds"] = round(elapsed, 6)
        row["gcc_exit"] = code
        match = re.search(r"^checksum = ([0-9a-fA-F]+)$", output_text, re.MULTILINE)
        if code or not match:
            row.update(status="gcc_reference_failed", gcc_checksum_output=output_text[-500:])
            report["results"].append(row)
            save()
            continue
        checksum = int(match.group(1), 16)
        row["gcc_checksum"] = f"{checksum:08x}"

        text = source.read_text(encoding="utf-8")
        ret = text.rfind("return 0;")
        if ret < 0:
            row["status"] = "unsupported_csmith_shape"
            report["results"].append(row)
            save()
            continue
        checked_source = case / "checksum-checked.c"
        replacement = f"return ((crc32_context ^ 0xFFFFFFFFUL) == 0x{checksum:x}ULL) ? 0 : 1;"
        checked_source.write_text(text[:ret] + replacement + text[ret + len("return 0;"):], encoding="utf-8")
        row["checked_source_sha256"] = sha256(checked_source)
        verify = case / "gcc-checksum-verify"
        code, elapsed, _ = run(
            [gcc, "-std=c11", "-O0", "-w", "-DCSMITH_MINIMAL", "-DNOT_PRINT_CHECKSUM", "-I", include, checked_source, "-o", verify],
            case / "gcc-verify-compile.log", args.timeout, case,
        )
        if code:
            row.update(status="gcc_verify_compile_failed", gcc_verify_compile_exit=code)
        else:
            code, elapsed, _ = run([verify], case / "gcc-verify.log", args.timeout, case)
            row["gcc_verify_exit"] = code
            if code:
                row["status"] = "gcc_checksum_verify_failed"
        if row["status"]:
            report["results"].append(row)
            save()
            continue

        target_obj = case / "test.sia"
        code, elapsed, _ = run(
            [compiler, "-DCSMITH_MINIMAL", "-DNOT_PRINT_CHECKSUM", "-I", include, "-o", target_obj, checked_source],
            case / "cosmic-compile.log", args.timeout, root,
        )
        row["cosmic_compile_seconds"] = round(elapsed, 6)
        if code:
            row.update(status="cosmic_compile_failed", cosmic_compile_exit=code)
        else:
            command = [runner, target_obj, *runtime_objects]
            code, elapsed, _ = run(command, case / "lightingmachine.log", args.timeout, root)
            row["sia_execute_seconds"] = round(elapsed, 6)
            row["sia_exit"] = code
            row["status"] = "pass" if code == 0 else "sia_execution_failed"
        report["results"].append(row)
        save()
    counts = {}
    for row in report["results"]:
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    report["counts"] = counts
    report["completed_at_utc"] = datetime.now(timezone.utc).isoformat()
    report["status"] = "completed"
    save()
    print(json.dumps({"status": report["status"], "counts": counts, "summary": str(summary)}, indent=2))


if __name__ == "__main__":
    main()
