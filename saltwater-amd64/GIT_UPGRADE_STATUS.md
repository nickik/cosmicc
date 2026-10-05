# Own-project Git backend upgrade

Cosmic 0.13 with backend 5c3dc85e79c6502e83e3210a9a29b5aabbc0d441 passes the same 211/220 native cases at O0 and O2 as the prior ff5de800 compiler. All six-mode per-case compile/run/output outcomes are unchanged. The nine diagnosed rejections remain; no accepted execution failures occur.

Compiler SHA256: 53e8d148f14fd2aaf072eff3c6613a7255ae1ac2e14d95d707788b567379a8cf. One immutable snapshot in /tmp/cosmicc-013-native-snapshot/cosmicc is shared across all runs. Durable results: tools/amd64/results/2026-10-05-013-git-upgrade-220.json.gz and comparison.json.

General native ELF/ABI/linking O2 passes. Incoming stdarg, register aggregates and MEMORY aggregate returns each pass eight Cosmic/host combinations and four reference builds using GCC13/Clang18 at O0/O2. Reports remain under /tmp/cosmicc-013-native-{abi,stdarg,register,return} and durable result JSON names.

The backend candidate starts from published b952544 and restores the existing System V variadic-count hook plus frame regression test. Backend gates pass 233 unit tests and 20 focused SIA compilation tests. Draft [backend PR 13](https://github.com/nickik/crainlift/pull/13) is prepared for consumer qualification and approval. No remote main merge, third-party dependency update or Rust upgrade is part of this evidence.
