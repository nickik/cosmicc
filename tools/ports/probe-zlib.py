#!/usr/bin/env python3
"""Compile unmodified upstream zlib's Z_SOLO core; never execute host zlib.
Usage: python3 tools/ports/probe-zlib.py SOURCE OUTPUT [COMPILER]
Z_SOLO is an upstream freestanding configuration, with caller allocation.
Each translation unit is probed independently; this is not a linked port.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

source = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
compiler = Path(sys.argv[3] if len(sys.argv) > 3 else "target/debug/cosmicc").resolve()
output.mkdir(parents=True, exist_ok=True)
units = "adler32 crc32 deflate infback inffast inflate inftrees trees zutil".split()
results = []
for unit in units:
    path = source / (unit + ".c")
    command = [str(compiler), "-D", "Z_SOLO=1", "-I", str(source), str(path), "-o", str(output / (unit + ".sia"))]
    try:
        run = subprocess.run(command, capture_output=True, text=True, timeout=30)
        diagnostic = run.stdout + run.stderr
        status = "compiled" if run.returncode == 0 else "failed"
    except subprocess.TimeoutExpired as error:
        diagnostic = "compilation exceeded 30 seconds\n"
        status = "timeout"
    (output / (unit + ".log")).write_text(diagnostic)
    results.append({"unit": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "status": status, "diagnostic": diagnostic.splitlines()[:3], "command": command})
(output / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
for result in results:
    print(result["unit"], result["status"], " | ".join(result["diagnostic"]))
print("Translation-unit compilation only; no linked or Lighting application acceptance.")
