#!/usr/bin/env python3
"""Run a pinned, portable slice of GCC's execute torture tests on SIA."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

PIN = "6f155cc3f5a2dff33afe6cc3ed6c2e0e605ae6a3"
SUITE = Path("gcc/testsuite/gcc.c-torture/execute")


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
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
        log.write_text(output)
        return process.returncode, time.monotonic() - started
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        log.write_text(output + "\nTIMEOUT\n")
        return 124, time.monotonic() - started


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gcc_source", type=Path, help="clean checkout of the pinned rust-lang/gcc mirror")
    parser.add_argument("output", type=Path, help="new output directory")
    parser.add_argument("--limit", type=int, default=100, help="number of sorted eligible execute cases (default: 100)")
    parser.add_argument("--compiler", type=Path, default=Path("target/debug/cosmicc"))
    parser.add_argument("--linker", type=Path, default=Path("target/debug/cosmic-link"))
    parser.add_argument("--timeout", type=int, default=10, help="seconds per subprocess (default: 10)")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    source_root = args.gcc_source.resolve()
    output = args.output.resolve()
    if args.limit <= 0 or args.timeout <= 0:
        parser.error("--limit and --timeout must be positive")
    if output.exists():
        parser.error(f"output directory already exists: {output}")
    suite = source_root / SUITE
    if not suite.is_dir():
        parser.error(f"missing GCC torture directory: {suite}")
    revision = subprocess.check_output(
        ["git", "-C", str(source_root), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != PIN:
        parser.error(f"suite revision is {revision}; expected pinned {PIN}")
    if subprocess.check_output(
        ["git", "-C", str(source_root), "status", "--porcelain"], text=True
    ).strip():
        parser.error("GCC checkout has uncommitted changes")
    output.mkdir(parents=True)
    cases_dir = output / "cases"
    cases_dir.mkdir()
    candidates, excluded = [], {}
    for path in sorted(suite.glob("*.c")):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            excluded["source is not UTF-8"] = excluded.get("source is not UTF-8", 0) + 1
            continue
        reason = None
        if "dg-" in text:
            reason = "DejaGNU directive or target-specific option"
        elif "#include" in text:
            reason = "header or system-library dependency"
        elif "main" not in text:
            reason = "no main entry point in standalone source"
        if reason:
            excluded[reason] = excluded.get(reason, 0) + 1
        else:
            candidates.append(path)
    selected = candidates[:args.limit]
    compiler = args.compiler if args.compiler.is_absolute() else root / args.compiler
    linker = args.linker if args.linker.is_absolute() else root / args.linker
    if not compiler.is_file() or not linker.is_file():
        parser.error("build target/debug/cosmicc and target/debug/cosmic-link before running")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1",
         "--manifest-path", str(root / "tools/l21-execution/Cargo.toml")], text=True
    ))
    runner = Path(metadata["target_directory"]) / "debug/multi-object-native"
    build_code, _ = run(
        ["cargo", "build", "--offline", "--locked", "--manifest-path",
         root / "tools/l21-execution/Cargo.toml", "--bin", "multi-object-native"],
        output / "runner-build.log", 600, root,
    )
    if build_code:
        raise SystemExit("failed to build multi-object-native; see runner-build.log")

    support = output / "support.c"
    wrapper = output / "wrapper.c"
    support.write_text(
        "void abort(void) {}\n"
        "void exit(int status) { (void)status; }\n"
    )
    wrapper.write_text(
        "extern int main(void);\n"
        "int __gcc_torture_entry(void) { return main() != 0; }\n"
    )
    runtime_objects = []
    for name, path, extra in (
        ("support", support, []), ("wrapper", wrapper, []),
    ):
        obj = output / f"{name}.sia"
        code, _ = run([compiler, *extra, path, "-o", obj], output / f"{name}-compile.log", args.timeout, root)
        if code:
            raise SystemExit(f"failed to compile {name}; see {name}-compile.log")
        runtime_objects.append(obj)
    softfloat_sources = [
        root / "runtime/softfloat/context.c",
        root / "runtime/softfloat/binary32-add.c",
        root / "runtime/softfloat/binary32-convert.c",
        root / "runtime/softfloat/binary64.c",
    ]
    for path in softfloat_sources:
        obj = output / f"runtime-{path.stem}.sia"
        code, _ = run(
            [compiler, "-I", root / "runtime/softfloat", path, "-o", obj],
            output / f"runtime-{path.stem}-compile.log", args.timeout, root,
        )
        if code:
            raise SystemExit(f"failed to compile soft-float runtime {path}; see runtime-{path.stem}-compile.log")
        runtime_objects.append(obj)

    report = {
        "status": "running", "started_at_utc": datetime.now(timezone.utc).isoformat(),
        "suite": "GCC gcc.c-torture/execute",
        "suite_revision": revision, "suite_commit_url":
        f"https://github.com/rust-lang/gcc/commit/{revision}",
        "selection": "first sorted direct .c cases without dg-* text, #include, or absent main token",
        "suite_case_count": len(list(suite.glob("*.c"))),
        "eligible_case_count": len(candidates), "selected_case_count": len(selected),
        "selection_exclusions": excluded, "host_reference": "GCC -std=gnu89 -O0",
        "target_execution": "LightingMachine simulator via checked CSIAIMG loader; integer-only binary32/binary64 runtime objects linked",
        "compiler_sha256": sha256(compiler), "linker_sha256": sha256(linker),
        "gcc_version": subprocess.check_output(["gcc", "--version"], text=True).splitlines()[0],
        "gcc_executable_sha256": sha256(Path(shutil.which("gcc")).resolve()),
        "runner_sha256": sha256(runner),
        "runner_script_sha256": sha256(Path(__file__).resolve()),
        "runtime_source_sha256": {
            str(path.relative_to(root)): sha256(path) for path in softfloat_sources
        },
        "results": [],
    }
    summary = output / "summary.json"
    summary.write_text(json.dumps(report, indent=2) + "\n")
    for number, path in enumerate(selected, 1):
        name = path.name
        case_dir = cases_dir / name.removesuffix(".c")
        case_dir.mkdir()
        row = {"name": name, "source_sha256": sha256(path)}
        native = case_dir / "gcc-reference"
        code, elapsed = run(
            ["gcc", "-std=gnu89", "-w", "-O0", path, "-o", native],
            case_dir / "gcc-compile.log", args.timeout,
        )
        row["gcc_compile_seconds"] = round(elapsed, 6)
        if code:
            row.update(status="gcc_compile_failed", gcc_compile_exit=code)
        else:
            code, elapsed = run([native], case_dir / "gcc-execute.log", args.timeout)
            row["gcc_execute_seconds"] = round(elapsed, 6)
            row["gcc_exit"] = code
            if code:
                row["status"] = "gcc_reference_failed"
        if row.get("status") is None:
            sia_object = case_dir / "test.sia"
            code, elapsed = run(
                [compiler, path, "-o", sia_object],
                case_dir / "sia-compile.log", args.timeout, root,
            )
            row["cosmic_compile_seconds"] = round(elapsed, 6)
            if code:
                row.update(status="cosmic_compile_failed", cosmic_compile_exit=code)
            else:
                image = case_dir / "image.csia"
                code, elapsed = run(
                    [linker, "--base", "0x10000", "--entry", "__gcc_torture_entry", "--max-bytes", "0xc0000",
                     "--container", "--gc-sections", "-o", image,
                     runtime_objects[1], sia_object, runtime_objects[0], *runtime_objects[2:]],
                    case_dir / "sia-link.log", args.timeout, root,
                )
                row["cosmic_link_seconds"] = round(elapsed, 6)
                if code:
                    row.update(status="cosmic_link_failed", cosmic_link_exit=code)
                else:
                    code, elapsed = run(
                        [runner, "--c-runtime-termination", "--image", image],
                        case_dir / "sia-execute.log", args.timeout,
                    )
                    row["sia_execute_seconds"] = round(elapsed, 6)
                    row["sia_exit"] = code
                    row["status"] = "pass" if code == 0 else "sia_execution_failed"
        report["results"].append(row)
        report["completed"] = number
        summary.write_text(json.dumps(report, indent=2) + "\n")
    counts = {}
    for row in report["results"]:
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    report["status"] = "completed"
    report["completed_at_utc"] = datetime.now(timezone.utc).isoformat()
    report["counts"] = counts
    report["warning"] = (
        "GCC torture is a regression corpus, not a standards-conformance certificate. "
        "A mismatch needs source/defined-behavior review; this filtered slice omits "
        "DejaGNU cases and tests requiring headers or system libraries."
    )
    summary.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"status": report["status"], "revision": revision,
                      "selected": len(selected), "counts": counts,
                      "summary": str(summary)}, indent=2))


if __name__ == "__main__":
    main()
