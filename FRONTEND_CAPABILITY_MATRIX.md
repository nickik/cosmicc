# Cosmic C frontend capability matrix

This matrix records frontend evidence separately from SIA lowering evidence.
“Supported” means the parser/preprocessor has a deterministic test in the
maintained workspace. It does not mean that the construct can be lowered to
COSMIC-SIA.

| Area | Construct | Classification | Evidence / boundary |
| --- | --- | --- | --- |
| Lexical | identifiers, keywords, integer/character/string literals, operators, comments | Supported | Parser and lexer unit tests; integer literals only reach the current SIA path. |
| Lexical | floating-point literals | Explicitly rejected for SIA | `saltwater-sia` rejects them before backend lowering. |
| Preprocessor | object-like macros | Supported | Deterministic fixture and parser tests. |
| Preprocessor | function-like macros | Supported | Deterministic fixture and parser tests. Variadic macros remain deferred. |
| Preprocessor | `#if`, `#elif`, `#else`, `#ifdef`, `#ifndef`, `defined` | Supported | Integer-expression and macro-selection tests. |
| Preprocessor | `#define` / `#undef` command-line definitions | Supported at parser API | `Opt.definitions` and source `#undef` are tested; CLI flags remain a driver task. |
| Preprocessor | quoted local `#include` | Supported | Checked-in fixture proves filename-relative lookup. |
| Preprocessor | system include search, `#pragma once`, `#line`, `#warning` | Parser-only / compatibility | Existing parser tests; no Cosmic sysroot contract yet. |
| Preprocessor | token pasting, stringification, variadic macros | Broken or deferred | No Cosmic profile evidence; must remain out of the supported claim. |
| Declarations | `char`, `short`, `int`, `long`, signedness, `_Bool`, pointers | Parser-only or partially lowerable | Parser accepts more than the current SIA lowering supports. |
| Declarations | structs, unions, enums, typedefs, qualifiers, storage classes | Parser-only / not yet lowerable | Frontend infrastructure exists; layout/ABI and object lowering are deferred. |
| Statements | compound blocks, declarations, expression statements, explicit `return` | Partially supported | Simple integer functions are covered by SIA tests. |
| Statements | `if`, loops, `switch`, labels, `goto`, `break`, `continue` | Parsed but not lowerable | M3 is blocked by SIA32 comparison/boolean lowering and control-flow work. |
| Expressions | integer literals, locals, assignment, `+ - & | ^ << >>`, unary integer operations | Supported in current SIA profile | `saltwater-sia` CLIF and emitted-byte tests. |
| Expressions | comparisons, logical operators, conditional expressions, calls | Parsed or semantically represented; not lowerable | No direct SIA fallback; comparisons await Cranelift `icmp`. |
| Initializers | scalar initialized SSA locals | Supported in current SIA profile | Existing SIA tests. |
| Initializers | aggregates, strings, compound literals, static constants | Parser-only / not yet lowerable | Requires memory, global data, and object-format work. |
| Target boundary | float/double, I64/`long long`, varargs, TLS, atomics | Explicitly deferred/rejected | No support or fallback is permitted. |
| Target boundary | globals, address-taken locals, loads/stores, objects/linking | Explicitly deferred | Requires M3.3/M5 contracts. |
| Execution | COSMIC-SIA bytes and `CallPlan` decoding | Supported as encoding evidence | Does not prove execution. |
| Execution | real Lighting board execution | Externally blocked | Waiting for Lighting’s public board-call runner. |

The matrix is intentionally conservative: a parser result is not a compiler
claim until the Cranelift SIA32 path and its tests prove the generated form.
