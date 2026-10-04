use lighting_simulation::native_board_call::{run_native_board_call, NativeBoardCall};

fn run(source: &str, args: &[u32]) -> u64 {
    let bridge = std::env::var("LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE").unwrap();
    let artifact = std::panic::catch_unwind(|| saltwater_sia::compile_default(source))
        .unwrap_or_else(|_| panic!("compiler panic for {source}"))
        .expect(source);
    let mut plan = artifact.prepare_integer_call("f", 0x8000, args).unwrap();
    plan.registers[13] = 0xf0000;
    let result = run_native_board_call(
        &bridge,
        &NativeBoardCall {
            code: &plan.code,
            entry: plan.entry_address,
            return_address: plan.return_address,
            registers: plan.registers,
            instruction_limit: 10000,
            cycle_limit: 100000,
        },
    )
    .expect(source);
    assert_eq!(result.state.registers[13], 0xf0000);
    u64::from(result.state.registers[1]) | (u64::from(result.state.registers[2]) << 32)
}
fn main() {
    let pairs = [
        (0u64, 0u64),
        (u32::MAX as u64, 1),
        (0x1_0000_0000, 1),
        (u64::MAX, 1),
        (0x1234_5678_9abc_def0, 0xfedc_ba98_7654_3210),
        (1, u64::MAX),
    ];
    let mut count = 0;
    for (x, y) in pairs {
        for op in ["+", "-", "&", "|", "^"] {
            let source = format!("unsigned long long f(unsigned long long x, unsigned long long y) {{ return x {op} y; }}");
            let expected = match op {
                "+" => x.wrapping_add(y),
                "-" => x.wrapping_sub(y),
                "&" => x & y,
                "|" => x | y,
                "^" => x ^ y,
                _ => unreachable!(),
            };
            assert_eq!(
                run(
                    &source,
                    &[x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32]
                ),
                expected,
                "{source}: {x:x}, {y:x}"
            );
            count += 1;
        }
    }
    for value in [0u32, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        assert_eq!(
            run("long long f(int x) { return x; }", &[value]),
            value as i32 as i64 as u64
        );
        assert_eq!(
            run("unsigned long long f(unsigned x) { return x; }", &[value]),
            value as u64
        );
        count += 2;
    }
    assert_eq!(
        run(
            "unsigned f(unsigned long long x) { return (unsigned)x; }",
            &[0x1234_5678, 0xdead_beef]
        ) as u32,
        0x1234_5678
    );
    for value in [
        0u64,
        0xffff_ffff,
        0x1_0000_0000,
        0x1234_5678_9abc_def0,
        u64::MAX,
    ] {
        let source = format!("unsigned long long f(void) {{ return 0x{value:x}ULL; }}");
        assert_eq!(run(&source, &[]), value);
        count += 1;
    }
    for value in [0u64, 1, 0x1234_5678_9abc_def0, u64::MAX] {
        for shift in [0u32, 1, 31, 32, 33, 63] {
            for (ty, op, expected) in [
                ("unsigned long long", "<<", value << shift),
                ("unsigned long long", ">>", value >> shift),
                ("long long", ">>", ((value as i64) >> shift) as u64),
            ] {
                let source = format!("{ty} f({ty} x) {{ return x {op} {shift}; }}");
                assert_eq!(
                    run(&source, &[value as u32, (value >> 32) as u32]),
                    expected,
                    "{source}: {value:x}"
                );
                count += 1;
            }
        }
    }
    for (x, y) in pairs {
        for unsigned in [false, true] {
            let ty = if unsigned {
                "unsigned long long"
            } else {
                "long long"
            };
            for op in ["==", "!=", "<", "<=", ">", ">="] {
                let (a, b) = if unsigned {
                    (x as i128, y as i128)
                } else {
                    (x as i64 as i128, y as i64 as i128)
                };
                let expected = match op {
                    "==" => a == b,
                    "!=" => a != b,
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    ">=" => a >= b,
                    _ => unreachable!(),
                };
                let source = format!("int f({ty} x, {ty} y) {{ return x {op} y; }}");
                assert_eq!(
                    run(
                        &source,
                        &[x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32]
                    ) as u32,
                    expected as u32,
                    "{source}: {x:x},{y:x}"
                );
                count += 1;
            }
        }
        let source = "unsigned long long f(unsigned long long x) { unsigned long long a[2]; a[1] = x; a[0] = x + 1; return a[1] + a[0]; }";
        assert_eq!(
            run(source, &[x as u32, (x >> 32) as u32]),
            x.wrapping_add(x.wrapping_add(1))
        );
        count += 1;
    }
    for value in [0u64, 1, 0xffff_ffff, 0x1234_5678_9abc_def0, u64::MAX] {
        let source = "unsigned long long f(unsigned long long x) { unsigned long long a = x; unsigned long long *p = &a; *p += 1; return a; }";
        assert_eq!(
            run(source, &[value as u32, (value >> 32) as u32]),
            value.wrapping_add(1)
        );
        count += 1;
    }
    for ty in ["char", "short"] {
        let source = format!("int f(int x) {{ {ty} a = x; {ty} *p = &a; *p = 7; return a; }}");
        assert_eq!(run(&source, &[1]) as u32, 7);
        count += 1;
        let source = format!("int f(int x) {{ {ty} a = x; {ty} *p = &a; *p = -7; return a; }}");
        assert_eq!(run(&source, &[1]) as u32, (-7i32) as u32);
        count += 1;
    }
    println!("PASS {count} native I64 arithmetic/bitwise/extension cases and low-word reduction");
}
