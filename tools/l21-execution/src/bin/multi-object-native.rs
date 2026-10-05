//! Execute independently serialized SIA C objects through public Lighting paths.
use saltwater_sia::{compile_default, Artifact};
fn main() {
    let board = std::env::args().any(|a| a == "--board");
    let c_runtime_termination = std::env::args().any(|a| a == "--c-runtime-termination");
    assert!(
        !board || !c_runtime_termination,
        "--c-runtime-termination is a LightingMachine simulator option"
    );
    let paths: Vec<_> = std::env::args()
        .skip(1)
        .filter(|a| a != "--board" && a != "--c-runtime-termination")
        .collect();
    let (mut image, object_count) = if paths.first().map(String::as_str) == Some("--image") {
        assert_eq!(paths.len(), 2, "--image expects one container");
        let bytes = std::fs::read(&paths[1]).unwrap();
        let image = if bytes.starts_with(b"\x7fELF") {
            saltwater_sia::LinkedImage::from_elf_executable_bytes(&bytes, 0xc0000).unwrap()
        } else {
            saltwater_sia::LinkedImage::from_image_bytes(&bytes, 0x800000, 0xc0000).unwrap()
        };
        (image, 0)
    } else {
        let mut objects = if paths.is_empty() {
            [
            "extern int get(void); extern char *message(void); static int value=7; static int helper(void){return value;} static char *text=\"b\"; int main(void){ if(get()!=3 || helper()!=7 || message()[0]!=97 || text[0]!=98) return 1; return 0;}",
            "static int value=3; static int helper(void){return value;} static char *text=\"a\"; char *message(void){return text;} int get(void){return helper();}"
        ].iter().map(|s| { let a=compile_default(s).unwrap(); Artifact::from_bytes(&a.to_bytes().unwrap()).unwrap() }).collect::<Vec<_>>()
        } else {
            paths
                .iter()
                .map(|p| Artifact::from_bytes(&std::fs::read(p).unwrap()).unwrap())
                .collect()
        };
        let index = objects
            .iter()
            .position(|o| o.function("main").is_some())
            .expect("main object");
        objects.swap(0, index);
        let index = objects[0]
            .functions
            .iter()
            .position(|f| f.name == "main")
            .unwrap();
        objects[0].functions.swap(0, index);
        (
            Artifact::link_images(&objects, 0x10000, "main", 0xc0000).unwrap(),
            objects.len(),
        )
    };
    let return_address = 0xd0000;
    let stack = 0xf0000;
    let instructions = if board {
        assert_eq!(image.base,image.entry,"board-call transport requires entry at image base; use architectural --image or explicit object relayout");
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
                instruction_limit: 30_000_000,
                cycle_limit: 600_000_000,
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
        // Dirty RAM makes zero-initialization independent of simulator defaults.
        let mut ram = vec![0xa5; 1024 * 1024];
        let startup = image.load_into(&mut ram, 0xe0000, stack, return_address).unwrap();
        machine.load_ram(0, &ram).unwrap();
        for register in 1..15 {
            machine.cpu_mut().core_mut().write_reg(register, startup.registers[register]);
        }
        machine.cpu_mut().core_mut().set_pc_for_test(startup.entry);
        let mut completed = None;
        let mut terminated_by_exit = false;
        let exit_address = image.symbols.get("exit").copied();
        let abort_address = image.symbols.get("abort").copied();
        let mut trace = std::collections::VecDeque::new();
        for count in 1..=30_000_000 {
            let pc = machine.cpu().core().pc();
            if c_runtime_termination && Some(pc) == abort_address {
                panic!("guest called abort(): {}", machine.register_dump());
            }
            if c_runtime_termination && Some(pc) == exit_address {
                assert_eq!(
                    machine.cpu().core().read_reg(1),
                    0,
                    "guest called exit with nonzero status: {}",
                    machine.register_dump()
                );
                terminated_by_exit = true;
                completed = Some(count);
                break;
            }
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
            trace.push_back(pc);
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
        let completed = completed.expect("instruction budget exceeded");
        if !terminated_by_exit {
            assert_eq!(machine.cpu().core().read_reg(13), stack, "stack not restored");
        }
        completed
    };

    if object_count == 0 {
        println!(
            "PASS loaded SIA image: {} image bytes; {instructions} instructions; board={board}",
            image.bytes.len()
        );
    } else {
        println!(
            "PASS multi-object: {object_count} objects; {instructions} instructions; board={board}"
        );
    }
}
