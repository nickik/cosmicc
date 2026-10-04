use lighting_simulation::native_board_call::{run_native_board_call, NativeBoardCall};
use saltwater_sia::compile_default as compile_source;

const STACK_TOP: u32 = 0xf0000;

fn main() {
    let bridge = std::env::var("LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE")
        .expect("set LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE to the real Bluesim bridge");
    // Address reuse detects leaks at intermediate scope boundaries, which a
    // final-SP-only assertion could miss. Bounds deliberately require rounding.
    let cases = [
        ("normal_scope", "int f(int n) { unsigned first = 0; int sum = 0; for (int i = 0; i < 32; i++) { int a[n]; if (i == 0) first = (unsigned)a; else if ((unsigned)a != first) return -1; a[n-1] = i; sum += a[n-1]; } return sum; }", vec![5], 496),
        ("nested_scope", "int f(int n) { int outer[n]; outer[0] = 13; unsigned first = 0; for (int i = 0; i < 16; i++) { int a[n+2]; int b[n+4]; if (i == 0) first = (unsigned)b; else if ((unsigned)b != first) return -1; b[n+3] = i; a[n+1] = b[n+3]; } return outer[0]; }", vec![3], 13),
        ("return_nested", "int f(int n) { int a[n]; a[0] = 17; { int b[n+2]; b[n+1] = 25; return a[0] + b[n+1]; } }", vec![3], 42),
        ("return_conditional", "int f(int n, int early) { int a[n]; a[0] = 9; if (early) return a[0]; { int b[n+2]; b[n+1] = 7; a[0] += b[n+1]; } return a[0]; }", vec![3, 0], 16),
        ("return_early", "int f(int n, int early) { int a[n]; a[0] = 9; if (early) return a[0]; { int b[n+2]; b[n+1] = 7; a[0] += b[n+1]; } return a[0]; }", vec![3, 1], 9),
        ("continue", "int f(int n) { unsigned first = 0; int sum = 0; for (int i = 0; i < 32; i++) { int a[n]; int b[n+2]; if (i == 0) first = (unsigned)b; else if ((unsigned)b != first) return -1; b[n+1] = i; sum += b[n+1]; continue; } return sum; }", vec![3], 496),
        ("break", "int f(int n) { unsigned first = 0; int sum = 0; for (int i = 0; i < 16; i++) { while (1) { int a[n]; int b[n+2]; if (i == 0) first = (unsigned)b; else if ((unsigned)b != first) return -1; b[n+1] = i; sum += b[n+1]; break; } } return sum; }", vec![3], 120),
        ("goto_out", "int f(int n) { unsigned first = 0; int i = 0; int sum = 0; again: { int a[n]; int b[n+2]; if (i == 0) first = (unsigned)b; else if ((unsigned)b != first) return -1; b[n+1] = i; sum += b[n+1]; goto outside; } outside: i++; if (i < 16) goto again; return sum; }", vec![3], 120),
        ("goto_same_scope", "int f(int n) { int a[n]; a[0] = 0; unsigned base = (unsigned)a; again: a[0]++; if ((unsigned)a != base) return -1; if (a[0] < 16) goto again; return a[0]; }", vec![5], 16),
        ("multidimensional", "int f(int n, int m) { int a[n][m]; a[n-1][m-1] = 23; a[0][0] = 19; return a[n-1][m-1] + a[0][0]; }", vec![3, 5], 42),
        ("sizeof", "unsigned f(int n) { int a[n+2]; return sizeof(a); }", vec![3], 20),
    ];
    let conditional_loop = "int f(int n, int early) { int outer[n]; outer[0] = 11; for (int i = 0; i < 4; i++) { if (early) return outer[0]; { int a[n+2]; a[n+1] = i; } if (i == 2) break; } return outer[0]; }";
    let mut cases = cases.to_vec();
    cases.push((
        "conditional_return_then_break",
        conditional_loop,
        vec![3, 0],
        11,
    ));
    cases.push((
        "conditional_return_in_loop",
        conditional_loop,
        vec![3, 1],
        11,
    ));
    cases.push(("loop_exits_preserve_outer", "int f(int n) { int outer[n]; outer[0] = 7; unsigned first = 0; int sum = 0; for (int i = 0; i < 4; i++) { int a[n+2]; if (i == 0) first = (unsigned)a; else if ((unsigned)a != first) return -1; a[n+1] = i; if (i == 1) continue; if (i == 2) break; sum += a[n+1]; } return outer[0] + sum; }", vec![3], 7));
    cases.push(("goto_preserves_outer", "int f(int n) { int outer[n]; outer[0] = 55; int i = 0; unsigned first = 0; again: { int a[n+2]; if (i == 0) first = (unsigned)a; else if ((unsigned)a != first) return -1; a[n+1] = i; goto outside; } outside: i++; if (i < 8) goto again; return outer[0]; }", vec![3], 55));
    for (name, source, args, expected) in cases {
        let artifact = compile_source(source).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut plan = artifact
            .prepare_integer_call("f", 0x8000, &args)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        if let Ok(directory) = std::env::var("COSMICC_L21_DUMP_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{name}.bin")),
                &plan.code,
            )
            .unwrap();
        }
        plan.registers[13] = STACK_TOP;
        let result = run_native_board_call(
            &bridge,
            &NativeBoardCall {
                code: &plan.code,
                entry: plan.entry_address,
                return_address: plan.return_address,
                registers: plan.registers,
                instruction_limit: 100_000,
                cycle_limit: 2_000_000,
            },
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            result.state.registers[1], expected,
            "{name}: {:?}",
            result.state
        );
        assert_eq!(
            result.state.registers[13], STACK_TOP,
            "{name}: stack not restored; {:?}",
            result.state
        );
        assert!(
            result.minimum_sp < STACK_TOP,
            "{name}: no stack allocation observed"
        );
        assert!(
            result.backend_transactions > 0,
            "{name}: no mainboard RAM traffic"
        );
        println!(
            "L21 PASS {name}: result={expected} SP={STACK_TOP:08x} instructions={} cycles={}",
            result.state.completed_instructions, result.cycles
        );
    }
    // Cover the comparison semantics required by loop/cleanup decisions.
    for operator in ["==", "!=", "<", "<=", ">", ">="] {
        for (left, right) in [(0u32, 0u32), (1, 2), (2, 1), (u32::MAX, 1)] {
            for unsigned in [false, true] {
                let ty = if unsigned { "unsigned" } else { "int" };
                let source = format!("int f({ty} a, {ty} b) {{ return a {operator} b; }}");
                let (a, b) = if unsigned {
                    (left as i64, right as i64)
                } else {
                    (left as i32 as i64, right as i32 as i64)
                };
                let expected = match operator {
                    "==" => a == b,
                    "!=" => a != b,
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    ">=" => a >= b,
                    _ => unreachable!(),
                } as u32;
                let artifact = compile_source(&source).unwrap();
                let mut plan = artifact
                    .prepare_integer_call("f", 0x8000, &[left, right])
                    .unwrap();
                plan.registers[13] = STACK_TOP;
                let result = run_native_board_call(
                    &bridge,
                    &NativeBoardCall {
                        code: &plan.code,
                        entry: plan.entry_address,
                        return_address: plan.return_address,
                        registers: plan.registers,
                        instruction_limit: 1000,
                        cycle_limit: 20_000,
                    },
                )
                .unwrap();
                assert_eq!(
                    result.state.registers[1], expected,
                    "{source}: a={left} b={right}"
                );
                assert_eq!(result.state.registers[13], STACK_TOP);
            }
        }
    }
    println!("L21 PASS 48 signed/unsigned comparison cases");
    let artifact = compile_source("int f(int n) { while (n) {} return 0; }").unwrap();
    let mut plan = artifact.prepare_integer_call("f", 0x8000, &[1]).unwrap();
    plan.registers[13] = STACK_TOP;
    let error = run_native_board_call(
        &bridge,
        &NativeBoardCall {
            code: &plan.code,
            entry: plan.entry_address,
            return_address: plan.return_address,
            registers: plan.registers,
            instruction_limit: 16,
            cycle_limit: 20_000,
        },
    )
    .unwrap_err();
    assert!(
        error.contains("instruction limit") && error.contains("trace=") && error.contains("state="),
        "{error}"
    );
    println!("L21 PASS bounded execution failure diagnostics");
    let artifact = compile_source("int f(int *p) { return *p; }").unwrap();
    let mut plan = artifact.prepare_integer_call("f", 0x8000, &[3]).unwrap();
    plan.registers[13] = STACK_TOP;
    let error = run_native_board_call(
        &bridge,
        &NativeBoardCall {
            code: &plan.code,
            entry: plan.entry_address,
            return_address: plan.return_address,
            registers: plan.registers,
            instruction_limit: 1000,
            cycle_limit: 20_000,
        },
    )
    .unwrap_err();
    assert!(
        error.contains("unexpected architectural trap") && error.contains("badaddr: 3"),
        "{error}"
    );
    println!("L21 PASS architectural fault diagnostics");
    let rejected = compile_source(
        "int f(int n) { goto inside; { int a[n]; inside: a[0] = 3; return a[0]; } }",
    )
    .unwrap_err();
    assert!(rejected
        .to_string()
        .contains("goto into a scope with active variable-length arrays"));
    println!("L21 PASS rejected entry into active VLA scope");
}
