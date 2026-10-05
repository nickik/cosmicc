# Active AMD64 continuation

- [x] Scalar ABI, narrow extensions, outgoing variadic AL counts and native floating point.
- [x] Incoming va_start/va_arg/va_copy/va_end, register-save and overflow traversal with bidirectional GCC/Clang acceptance.
- [x] Large MEMORY aggregate parameters and hidden-destination returns.
- [x] Natural <=16-byte INTEGER/SSE aggregate parameters/returns, including register exhaustion rollback and aggregate va_arg.
- [x] Target math/string/stdio header declarations for hosted suite cases 174/179/186/187/189.
- [x] Permit nonvoid scalar fallthrough when its result is discarded (case 218); a used result has undefined C behavior.
- [x] Final all-220 matrix: 210 expected-output passes at both O0/O2, 9 compile rejections and no accepted execution failures. Preserve constraint diagnostics and classify GNU extensions separately.
- [x] Case 207: evaluate unused VLA bounds while omitting dead storage; preallocate fixed-size automatic storage so forward goto can reach array uses.
- [ ] Referenced native VLA storage/lifetimes and dynamic typedef/pointer bound capture; these remain diagnosed.
- [x] Shared frontend fixes 104/201/205/220 accepted in the final matrix.
- [x] C11 _Generic (219), including distinct plain/signed char identity: full matrix and focused six-mode execution pass.
- [ ] Native long double/x87 and long-double aggregate/variadic ABI (204).
- [ ] Optional explicit GNU compatibility: forward enums (170/209), statement expressions/branch hint (213/214), empty structures (216).
- [ ] Constraint boundaries 95/144/210: keep diagnostics; do not silently relax pointer or qualifier rules.
- [ ] Implement actual bit-field layout and access before claiming external ABI compatibility for them.
- [ ] Extend packed/overaligned/x87/vector classes only with a documented classifier and interop proof.
- [ ] Optimize incoming register saves using AL after acceptance gates.

Accepted native ABI details and durable scripts: STDARG_STATUS.md and REGISTER_AGGREGATE_STATUS.md. SIA has its own conventions and execution gates.

Current native crate gate: 19 tests pass. General ABI O2 passed current compiler SHA256 ff5de80019013a5315083698fc3a09290f787ffcb3be7145abbcd3a385ec5f0e; the unused-VLA and bidirectional stdarg/aggregate gates remain recorded against the prior f569 compiler. The final all-case differential snapshot passed 211/220 cases at O0 and O2. Raw records are retained in tools/amd64/results/2026-10-05-native-generic-220.json.gz and TSV.

Remaining rejected cases: 00095,00144,00170,00204,00209,00210,00213,00214,00216. Reference GCC/Clang agreement is differential evidence, not proof that a constraint-violating or undefined case is valid ISO C. Case 144 reads an uninitialized value as well as discarding const qualification.

Own-project Git upgrade replay: Cosmic 0.13 with backend 5c3dc85 retains 211/220 at both modes with zero outcome differences and all native ABI gates passing. See [upgrade evidence](GIT_UPGRADE_STATUS.md). Third-party packages and Rust are unchanged.
