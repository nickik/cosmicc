//! Execute compiler-produced upstream zlib CRC code on Lighting. Host-side
//! integer CRC only creates fixture expectations; it never services guest calls.
use lighting_simulation::isa::machine::LightingMachine;
use saltwater_sia::{compile, Opt};
use std::{fmt::Write, path::PathBuf};

fn fixture() -> String {
    let mut source = String::from("#define Z_SOLO 1\n#include \"crc32.c\"\nint main(void) { unsigned char data[257]; unsigned i; unsigned crc;\n");
    source.push_str("if (crc32(0, (const unsigned char *)0, 0) != 0) return 1;\n");
    source.push_str(
        "if (crc32(0, (const unsigned char *)\"123456789\", 9) != 0xcbf43926u) return 2;\n",
    );
    source.push_str(
        "if (crc32(0, (const unsigned char *)\"hello world\", 11) != 0x0d4a1185u) return 3;\n",
    );
    source.push_str("crc = 0; for (i = 0; i < 9; i++) crc = crc32(crc, (const unsigned char *)\"123456789\" + i, 1); if (crc != 0xcbf43926u) return 4;\n");
    source.push_str("for (i = 0; i < 257; i++) data[i] = (unsigned char)(i * 37u + 11u);\n");
    for (case, (start, length)) in [(0usize, 0usize), (0, 1), (0, 7), (0, 8), (0, 255), (1, 256)]
        .into_iter()
        .enumerate()
    {
        let mut crc = !0u32;
        for i in start..start + length {
            crc ^= ((i * 37 + 11) & 255) as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
            }
        }
        let expected = !crc;
        writeln!(
            source,
            "if (crc32(0, data + {start}, {length}) != {expected}u) return {};",
            case + 10
        )
        .unwrap();
    }
    source.push_str("return 0; }\n");
    source
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (bytes, base, entry, expected, board) = if args.first().map(String::as_str)
        == Some("--compile")
    {
        let directory = PathBuf::from(args.get(1).expect("--compile UPSTREAM_ZLIB_DIRECTORY"));
        let mut opt = Opt::default();
        opt.search_path.push(directory.clone());
        opt.filename = directory.join("cosmic-crc-fixture.c");
        let mut artifact =
            compile(&fixture(), opt).expect("compile unmodified zlib CRC with target compiler");
        let main_index = artifact
            .functions
            .iter()
            .position(|function| function.name == "main")
            .unwrap();
        artifact.functions.swap(0, main_index);
        let image = artifact
            .link_image(0x10000, "main", 0x40000)
            .expect("link CRC fixture");
        (
            image.bytes,
            image.base,
            image.entry,
            0,
            args.iter().any(|arg| arg == "--board"),
        )
    } else {
        let path = args
            .first()
            .expect("IMAGE ENTRY_HEX [EXPECTED_HEX] or --compile ZLIB_DIRECTORY [--board]");
        let entry = u32::from_str_radix(args.get(1).expect("entry hex"), 16).expect("entry hex");
        let expected = args
            .get(2)
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .unwrap_or(0xcbf43926);
        (
            std::fs::read(path).expect("image bytes"),
            0x10000,
            entry,
            expected,
            false,
        )
    };
    let input = 0x70000;
    let return_address = 0x80000;
    let stack = 0xf0000;
    let instructions = if board {
        use lighting_simulation::native_board_call::{run_native_board_call, NativeBoardCall};
        assert_eq!(entry, base, "public runner loads the image at its entry");
        let bridge =
            std::env::var("LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE").expect("set bridge path");
        let mut registers = [0; 16];
        registers[13] = stack;
        registers[14] = return_address;
        let result = run_native_board_call(
            &bridge,
            &NativeBoardCall {
                code: &bytes,
                entry,
                return_address,
                registers,
                instruction_limit: 1_000_000,
                cycle_limit: 20_000_000,
            },
        )
        .expect("composed-board CRC execution");
        assert_eq!(
            result.state.registers[1], expected,
            "CRC fixture result: {:?}",
            result.state
        );
        assert_eq!(result.state.registers[13], stack, "stack restoration");
        assert_eq!(
            result.state.registers[15], 0,
            "caller frame pointer restoration"
        );
        assert!(result.backend_transactions > 0);
        result.state.completed_instructions
    } else {
        let mut machine = LightingMachine::new(1024 * 1024, &[]).unwrap();
        machine.load_ram(base, &bytes).unwrap();
        machine.load_ram(input, b"123456789").unwrap();
        machine.cpu_mut().core_mut().set_pc_for_test(entry);
        for (register, value) in [
            (1, 0),
            (2, input),
            (3, 9),
            (13, stack),
            (14, return_address),
        ] {
            machine.cpu_mut().core_mut().write_reg(register, value);
        }
        let mut trace = std::collections::VecDeque::new();
        let mut completed = None;
        for instructions in 1..=1_000_000 {
            let pc = machine.cpu().core().pc();
            assert!(
                pc >= base && u64::from(pc) < u64::from(base) + bytes.len() as u64,
                "PC outside image: {}; trace={trace:?}",
                machine.register_dump()
            );
            let offset = (pc - base) as usize;
            let word = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let event = format!(
                "{instructions}: {} SP={:08x} FP={:08x} LR={:08x}",
                machine.cpu().core().disassemble_at(word, pc),
                machine.cpu().core().read_reg(13),
                machine.cpu().core().read_reg(15),
                machine.cpu().core().read_reg(14)
            );
            if std::env::var_os("COSMIC_CRC_TRACE").is_some() {
                eprintln!("{event}\n{}", machine.register_dump());
            }
            if trace.len() == 32 {
                trace.pop_front();
            }
            trace.push_back(event);
            machine.step_detailed().expect("Lighting guest execution");
            assert_eq!(
                machine.cpu().privileged_state().cause,
                0,
                "architectural trap: {}; trace={trace:?}",
                machine.register_dump()
            );
            if machine.cpu().core().pc() == return_address {
                assert_eq!(
                    machine.cpu().core().read_reg(1),
                    expected,
                    "CRC fixture result; trace={trace:?}"
                );
                assert_eq!(
                    machine.cpu().core().read_reg(13),
                    stack,
                    "stack restoration"
                );
                assert_eq!(
                    machine.cpu().core().read_reg(15),
                    0,
                    "caller frame pointer restoration"
                );
                completed = Some(instructions);
                break;
            }
        }
        completed.expect("CRC instruction budget exceeded")
    };
    println!("PASS unmodified upstream zlib CRC fixture; {instructions} guest instructions; board={board}");
}
