//! Execute linked integer-only C runtime foundations on Lighting. Host arithmetic
//! generates expected fixtures; it never services guest calls or memory accesses.
use saltwater_sia::compile_default;
use std::fmt::Write;

fn source() -> String {
    let mut source = include_str!("../../../../runtime/primitives.c").to_owned();
    source.push_str("\ncosmic_u32 div_cases[41][8] = {\n");
    let mut pairs = vec![
        (0u64, 1u64),
        (1, 1),
        (u64::MAX, 1),
        (u64::MAX, u64::MAX),
        (u64::MAX, 0x8000000000000001),
        (0x8000000000000000, 3),
        (0x100000000, 0xffffffff),
        (5, 8),
        (u64::MAX, 0x100000000),
    ];
    let mut state = 0x8192aabbccdd1234u64;
    for _ in 0..32 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let numerator = state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let denominator = state | 1;
        pairs.push((numerator, denominator));
    }
    for &(n, d) in &pairs {
        writeln!(
            source,
            "{{{}u,{}u,{}u,{}u,{}u,{}u,{}u,{}u}},",
            n as u32,
            (n >> 32) as u32,
            d as u32,
            (d >> 32) as u32,
            (n / d) as u32,
            ((n / d) >> 32) as u32,
            (n % d) as u32,
            ((n % d) >> 32) as u32
        )
        .unwrap();
    }
    source.push_str("};\ncosmic_u32 jam_cases[50][5] = {\n");
    for value in [0, 1, 0x8000000000000000, 0x123456789abcdef0, u64::MAX] {
        for distance in [0u32, 1, 7, 31, 32, 33, 63, 64, 65, u32::MAX] {
            let expected = if distance == 0 {
                value
            } else if distance >= 64 {
                u64::from(value != 0)
            } else {
                (value >> distance) | u64::from(value & ((1u64 << distance) - 1) != 0)
            };
            writeln!(
                source,
                "{{{}u,{}u,{}u,{}u,{}u}},",
                value as u32,
                (value >> 32) as u32,
                distance,
                expected as u32,
                (expected >> 32) as u32
            )
            .unwrap();
        }
    }
    source.push_str("};\ncosmic_u32 mul_cases[25][4] = {\n");
    for a in [0u32, 1, 0x80000000, 0xffffffff, 0x12345678] {
        for b in [0u32, 1, 0x80000000, 0xffffffff, 0x87654321] {
            let expected = u64::from(a) * u64::from(b);
            writeln!(
                source,
                "{{{a}u,{b}u,{}u,{}u}},",
                expected as u32,
                (expected >> 32) as u32
            )
            .unwrap();
        }
    }
    source.push_str(r#"};
struct OffsetInner { char tag; int value; };
struct OffsetOuter { char tag; struct OffsetInner inner; };
struct OffsetArray { char tag; int data[3]; };
unsigned offset_nested(void) { return (unsigned)&((struct OffsetOuter *)0)->inner.value; }
unsigned offset_array(void) { return (unsigned)&((struct OffsetArray *)0)->data[2]; }
int qualifier_probe(int *p) { const int *q = p; q = p; return *q; }
int value = 7;
int *value_ptr = &value;
int get_value(void) { return *value_ptr; }
int (*get_value_ptr)(void) = get_value;
int main(void) {
    cosmic_words64 q, r;
    unsigned char bytes[9], copy[9];
    cosmic_u32 i;
    if (get_value_ptr() != 7 || get_value() != 7) return 1;
    if (offset_nested() != 8) return 1000 + offset_nested();
    if (offset_array() != 12) return 2000 + offset_array();
    if (qualifier_probe(&value) != 7) return 25;
    cosmic_memset(bytes, 0xab, 9);
    cosmic_memcpy(copy, bytes, 9);
    if (cosmic_memcmp(bytes, copy, 9) != 0) return 2;
    copy[8] = 0xff;
    if (cosmic_memcmp(bytes, copy, 9) >= 0) return 3;
    if (cosmic_memcmp(copy, bytes, 9) <= 0) return 4;
    for (i = 0; i < 9; i++) bytes[i] = (unsigned char)i;
    cosmic_memmove(bytes + 1, bytes, 8);
    for (i = 1; i < 9; i++) if (bytes[i] != i - 1) return 5;
    cosmic_memmove(bytes, bytes + 1, 8);
    for (i = 0; i < 8; i++) if (bytes[i] != i) return 6;
    cosmic_memmove(bytes, bytes, 9);
    cosmic_memset(bytes, 0, 0);
    if (bytes[7] != 7 || cosmic_memcmp(bytes, bytes, 0) != 0) return 7;
    if (cosmic_f32_neg_bits(0) != 0x80000000u) return 8;
    if (cosmic_f32_abs_bits(0xffc01234u) != 0x7fc01234u) return 9;
    if (cosmic_f32_class_bits(0x80000000u) != 0) return 10;
    if (cosmic_f32_class_bits(1) != 1) return 11;
    if (cosmic_f32_class_bits(0x3f800000u) != 2) return 12;
    if (cosmic_f32_class_bits(0xff800000u) != 3) return 13;
    if (cosmic_f32_class_bits(0x7f800001u) != 4) return 14;
    if (cosmic_f64_class_words(0, 0x80000000u) != 0) return 15;
    if (cosmic_f64_class_words(1, 0) != 1) return 16;
    if (cosmic_f64_class_words(0, 0x3ff00000u) != 2) return 17;
    if (cosmic_f64_class_words(0, 0xfff00000u) != 3) return 18;
    if (cosmic_f64_class_words(1, 0x7ff00000u) != 4) return 19;
    if (cosmic_shift_right_jam32(0x80000001u, 1) != 0x40000001u) return 20;
    if (cosmic_shift_right_jam32(0x80000000u, 0) != 0x80000000u) return 21;
    if (cosmic_shift_right_jam32(0x80000000u, 32) != 1) return 22;
    if (cosmic_shift_right_jam32(0, 0xffffffffu) != 0) return 23;
    for (i = 0; i < sizeof(jam_cases) / sizeof(jam_cases[0]); i++) {
        cosmic_shift_right_jam64(&q, jam_cases[i][0], jam_cases[i][1], jam_cases[i][2]);
        if (q.low != jam_cases[i][3] || q.high != jam_cases[i][4]) return 100 + i;
    }
    for (i = 0; i < sizeof(mul_cases) / sizeof(mul_cases[0]); i++) {
        cosmic_mul32_wide(&q, mul_cases[i][0], mul_cases[i][1]);
        if (q.low != mul_cases[i][2] || q.high != mul_cases[i][3]) return 200 + i;
    }
    for (i = 0; i < sizeof(div_cases) / sizeof(div_cases[0]); i++) {
        if (cosmic_udivmod64(&q, &r, div_cases[i][0], div_cases[i][1], div_cases[i][2], div_cases[i][3]) != 0) return 300 + i;
        if (q.low != div_cases[i][4] || q.high != div_cases[i][5] ||
            r.low != div_cases[i][6] || r.high != div_cases[i][7]) return 400 + i;
    }
    q.low = 11; q.high = 12; r.low = 13; r.high = 14;
    if (cosmic_udivmod64(&q, &r, 99, 0, 0, 0) != 1) return 500;
    if (q.low != 11 || q.high != 12 || r.low != 13 || r.high != 14) return 501;
    return 0;
}
"#);
    source
}

fn main() {
    let board = std::env::args().any(|argument| argument == "--board");
    let mut artifact = compile_default(&source()).expect("compile runtime guest");
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
                instruction_limit: 2_000_000,
                cycle_limit: 40_000_000,
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
        for count in 1..=2_000_000 {
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
    println!("PASS linked runtime: memory, FP bit primitives, 50 sticky shifts, 25 wide products, 41 divisions, zero-divisor handling and code/data/function-pointer relocations; {instructions} instructions; board={board}");
}
