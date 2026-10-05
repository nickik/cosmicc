//! Execute I32-only integer64 helpers; host arithmetic generates fixtures only.
use saltwater_sia::compile_default;
use std::fmt::Write;
fn source(board: bool) -> String {
    let mut source = include_str!("../../../../runtime/primitives.c").to_owned();
    source.push_str(include_str!("../../../../runtime/integer64.c"));
    let values = [
        i64::MIN,
        i64::MIN + 1,
        -0x100000001,
        -7,
        -1,
        0,
        1,
        3,
        0x100000001,
        i64::MAX,
    ];
    let mut cases = Vec::new();
    for n in values {
        for d in values {
            if d != 0 && !(n == i64::MIN && d == -1) {
                cases.push((n, d));
            }
        }
    }
    if board {
        cases = vec![
            (i64::MIN, 3),
            (i64::MIN, i64::MIN),
            (i64::MIN, i64::MIN + 1),
            (i64::MIN + 1, -7),
            (i64::MAX, -3),
            (-7, 3),
            (7, -3),
            (-1, i64::MIN),
        ];
    }
    writeln!(
        source,
        "\n#define RUN_HELPERS {}",
        if board { 0 } else { 1 }
    )
    .unwrap();
    writeln!(
        source,
        "\ncosmic_u32 signed_cases[{}][10] = {{",
        cases.len()
    )
    .unwrap();
    for (n, d) in cases {
        let q = (n / d) as u64;
        let r = (n % d) as u64;
        let ur = (n as u64) % (d as u64);
        writeln!(
            source,
            "{{{}u,{}u,{}u,{}u,{}u,{}u,{}u,{}u,{}u,{}u}},",
            n as u32,
            (n as u64 >> 32) as u32,
            d as u32,
            (d as u64 >> 32) as u32,
            q as u32,
            (q >> 32) as u32,
            r as u32,
            (r >> 32) as u32,
            ur as u32,
            (ur >> 32) as u32
        )
        .unwrap();
    }
    source.push_str("};\ncosmic_u32 product_cases[100][6] = {\n");
    for a in values {
        for b in values {
            let p = (a as u64).wrapping_mul(b as u64);
            writeln!(
                source,
                "{{{}u,{}u,{}u,{}u,{}u,{}u}},",
                a as u32,
                (a as u64 >> 32) as u32,
                b as u32,
                (b as u64 >> 32) as u32,
                p as u32,
                (p >> 32) as u32
            )
            .unwrap();
        }
    }
    source.push_str(r#"};
long long automatic_signed_remainder(long long n, long long d) { return n % d; }
unsigned long long automatic_unsigned_remainder(unsigned long long n, unsigned long long d) { return n % d; }
int main(void) {
    cosmic_words64 q, r;
    cosmic_u32 i;
    for (i=0; i<sizeof(signed_cases)/sizeof(signed_cases[0]); i++) {
        if (RUN_HELPERS) {
        if (cosmic_sdivmod64(&q,&r,signed_cases[i][0],signed_cases[i][1],signed_cases[i][2],signed_cases[i][3])) return 100+i;
        if (q.low != signed_cases[i][4] || q.high != signed_cases[i][5] || r.low != signed_cases[i][6] || r.high != signed_cases[i][7]) return 200+i;
        }
        {
            unsigned long long n = ((unsigned long long)signed_cases[i][1] << 32) | signed_cases[i][0];
            unsigned long long d = ((unsigned long long)signed_cases[i][3] << 32) | signed_cases[i][2];
            unsigned long long got = (unsigned long long)automatic_signed_remainder((long long)n, (long long)d);
            if ((unsigned)got != signed_cases[i][6] || (unsigned)(got >> 32) != signed_cases[i][7]) return 500+i;
            got = automatic_unsigned_remainder(n,d);
            if ((unsigned)got != signed_cases[i][8] || (unsigned)(got >> 32) != signed_cases[i][9]) return 600+i;
        }
    }
    if (RUN_HELPERS) {
    for (i=0; i<100; i++) {
        cosmic_mul64(&q,product_cases[i][0],product_cases[i][1],product_cases[i][2],product_cases[i][3]);
        if (q.low != product_cases[i][4] || q.high != product_cases[i][5]) return 300+i;
    }
    q.low=11; q.high=12; r.low=13; r.high=14;
    if (cosmic_sdivmod64(&q,&r,5,0,0,0)!=1) return 401;
    if (q.low!=11 || q.high!=12 || r.low!=13 || r.high!=14) return 402;
    if (cosmic_sdivmod64(&q,&r,0,0x80000000u,0xffffffffu,0xffffffffu)!=2) return 403;
    if (q.low!=11 || q.high!=12 || r.low!=13 || r.high!=14) return 404;
    }
    return 0;
}
"#);
    source
}

fn verify_remainder_traps() {
    use lighting_simulation::isa::machine::LightingMachine;
    for args in ["1LL,0LL", "(-9223372036854775807LL-1),-1LL"] {
        let source = format!("long long rem(long long n,long long d) {{ return n % d; }} int main(void) {{ return (int)rem({args}); }}");
        let artifact = compile_default(&source).expect("compile remainder trap fixture");
        let image = artifact.link_image(0x10000, "main", 0x10000).unwrap();
        let mut machine = LightingMachine::new(1024 * 1024, &[]).unwrap();
        machine.load_ram(image.base, &image.bytes).unwrap();
        machine.cpu_mut().core_mut().set_pc_for_test(image.entry);
        machine.cpu_mut().core_mut().write_reg(13, 0xf0000);
        machine.cpu_mut().core_mut().write_reg(14, 0x80000);
        let mut trapped = false;
        for _ in 0..1000 {
            machine.step_detailed().unwrap();
            if machine.cpu().privileged_state().cause != 0 {
                trapped = true;
                break;
            }
            assert_ne!(
                machine.cpu().core().pc(),
                0x80000,
                "invalid remainder returned normally"
            );
        }
        assert!(trapped, "invalid remainder did not trap: {args}");
    }
}

fn main() {
    let board = std::env::args().any(|argument| argument == "--board");
    let mut artifact = compile_default(&source(board)).expect("compile runtime guest");
    // The existing public board-call API loads code at its entry. Place main
    // first so this flat image uses that API without a private board transport.
    let main_index = artifact
        .functions
        .iter()
        .position(|function| function.name == "main")
        .unwrap();
    artifact.functions.swap(0, main_index);
    let mut image = artifact
        .link_image(0x10000, "main", 0x40000)
        .expect("link runtime guest");
    let return_address = 0x80000;
    let stack = 0xf0000;
    let instructions = if board {
        use lighting_simulation::native_board_call::{run_native_board_call, NativeBoardCall};
        let bridge = std::env::var("LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE")
            .expect("set Bluesim bridge path");
        while image.bytes.len() % 4 != 0 {
            image.bytes.push(0);
        }
        let mut registers = [0; 16];
        registers[13] = stack;
        registers[14] = return_address;
        let result = run_native_board_call(
            &bridge,
            &NativeBoardCall {
                code: &image.bytes,
                entry: image.entry,
                return_address,
                registers,
                instruction_limit: 5_000_000,
                cycle_limit: 100_000_000,
            },
        )
        .expect("execute board runtime guest");
        assert_eq!(
            result.state.registers[1], 0,
            "guest case failure: {:?}",
            result.state
        );
        assert_eq!(result.state.registers[13], stack, "stack not restored");
        assert!(result.backend_transactions > 0);
        result.state.completed_instructions
    } else {
        use lighting_simulation::isa::machine::LightingMachine;
        let mut machine = LightingMachine::new(1024 * 1024, &[]).unwrap();
        machine.load_ram(image.base, &image.bytes).unwrap();
        machine.cpu_mut().core_mut().set_pc_for_test(image.entry);
        machine.cpu_mut().core_mut().write_reg(13, stack);
        machine.cpu_mut().core_mut().write_reg(14, return_address);
        let mut completed = None;
        let mut trace = std::collections::VecDeque::new();
        for count in 1..=5_000_000 {
            let pc = machine.cpu().core().pc();
            assert!(
                image.regions.iter().any(|region| region.executable
                    && pc >= region.address
                    && pc < region.address + region.size),
                "PC outside linked code: {}; trace={:?}",
                machine.register_dump(),
                trace
            );
            if trace.len() == 24 {
                trace.pop_front();
            }
            trace.push_back(machine.register_dump());
            machine
                .step_detailed()
                .expect("execute Lighting runtime guest");
            assert_eq!(
                machine.cpu().privileged_state().cause,
                0,
                "architectural trap: {}; trace={:?}",
                machine.register_dump(),
                trace
            );
            if machine.cpu().core().pc() == return_address {
                assert_eq!(
                    machine.cpu().core().read_reg(1),
                    0,
                    "guest case failure: {}; trace={:?}; regions={:?}",
                    machine.register_dump(),
                    trace,
                    image.regions
                );
                assert_eq!(
                    machine.cpu().core().read_reg(13),
                    stack,
                    "stack not restored"
                );
                completed = Some(count);
                break;
            }
        }
        completed.expect("instruction budget exceeded")
    };
    if !board {
        verify_remainder_traps();
    }
    println!("PASS integer64: automatic signed/unsigned I64 remainders; {} cases each; {instructions} instructions; board={board}; helper fixtures={}", if board {8} else {89}, !board);
}
