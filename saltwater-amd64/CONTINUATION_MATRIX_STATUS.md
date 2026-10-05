# Native continuation matrix

The final immutable debug compiler ff5de80019013a5315083698fc3a09290f787ffcb3be7145abbcd3a385ec5f0e passes 211 of the pinned 220 cases at both O0 and O2. All accepted cases link and match expected output; nine cases are rejected at compilation. GCC13 and Clang18 match expected output for all 220, using the inventory's documented GNU/reference diagnostic settings.

Evidence: /tmp/cosmic-native-generic-final-220/summary.json. Durable compressed records and TSV: tools/amd64/results/2026-10-05-native-generic-220.*. This matrix includes the shared frontend literal, macro, array and UTF-32 fixes plus native header, aggregate, stdarg and storage changes. It establishes a tested subset, not full language or ABI conformance.

| Rejections | Boundary |
|---|---|
| 95, 144, 210 | Function/object-pointer implicit conversions or discarded qualifiers; 144 additionally reads an uninitialized local. Keep diagnostics. |
| 170, 209 | GNU forward enum declarations. |
| 204 | Genuine native extended-precision long-double/x87 support, including aggregates and variadics. |
| 213, 214 | GNU statement expressions; 214 also uses a branch hint builtin. |
| 216 | GNU empty structures. |

The native crate passes 19 tests. The current general ABI O2 gate and unchanged case 219 pass with this compiler. The previous 210/220 f569 matrix and its ABI interop records remain preserved under native-continuation result names. Incoming stdarg, register aggregates and MEMORY returns each pass eight Cosmic/host combinations and four references against the preserved previous f569 compiler. General ABI/linking O2 and unused-VLA bound/goto storage gates also pass. Detailed supported ABI classes and limits remain in STDARG_STATUS.md and REGISTER_AGGREGATE_STATUS.md.
