//! Focused bool-postfix regressions executed by Lighting guest instructions.
use saltwater_sia::compile_default;
fn source() -> String {
    include_str!("../../../amd64/bool-postfix.c").to_owned()
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
    println!("PASS bool-postfix canonical SSA/local/global/array/struct storage; {instructions} instructions; board={board}");
}
