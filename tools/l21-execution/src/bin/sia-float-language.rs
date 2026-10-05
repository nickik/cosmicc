//! Compile an unchanged C file and execute it on LightingMachine.

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let budget = args
        .iter()
        .position(|a| a == "--max-instructions")
        .map(|i| {
            args.get(i + 1)
                .expect("instruction budget value")
                .parse::<usize>()
                .expect("integer instruction budget")
        })
        .unwrap_or(2_000_000);
    assert!(
        (1..=20_000_000).contains(&budget),
        "instruction budget must be 1..=20000000"
    );
    let board = args.iter().any(|argument| argument == "--board");
    let path = std::env::args().nth(1).expect("source path");
    let source = std::fs::read_to_string(&path).unwrap();
    let source_opt = saltwater_sia::Opt {
        filename: path.clone().into(),
        ..Default::default()
    };
    let mut objects =
        vec![saltwater_sia::compile(&source, source_opt).expect("compile runtime guest")];
    let mut runtime_paths: Vec<String> = [
        "runtime/softfloat/context.c",
        "runtime/softfloat/fenv.c",
        "runtime/softfloat/binary32-add.c",
        "runtime/softfloat/binary32-convert.c",
        "runtime/softfloat/binary64.c",
        "runtime/softfloat/math-bits.c",
        "runtime/softfloat/math.c",
        "runtime/softfloat/math-extra.c",
        "runtime/decimal/parse.c",
        "runtime/decimal/format.c",
        "runtime/softfloat/math-transcendental.c",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for entry in
        std::fs::read_dir("runtime/softfloat/fdlibm/adapted").expect("fdlibm source directory")
    {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) == Some("c") {
            runtime_paths.push(path.to_string_lossy().into_owned());
        }
    }
    runtime_paths.sort();
    for path in runtime_paths {
        let code = std::fs::read_to_string(&path).unwrap();
        let opt = saltwater_sia::Opt {
            filename: path.clone().into(),
            ..Default::default()
        };
        let runtime =
            saltwater_sia::compile(&code, opt).expect("compile integer-only softfloat runtime");
        std::fs::create_dir_all("/tmp/cosmic-sia-float-runtime-objects").unwrap();
        let stem = std::path::Path::new(&path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        std::fs::write(
            format!("/tmp/cosmic-sia-float-runtime-objects/{stem}.sia"),
            runtime.to_bytes().unwrap(),
        )
        .unwrap();
        objects.push(runtime);
    }
    let mut artifact = objects.remove(0);
    // The existing public board-call API loads code at its entry. Place main
    // first so this flat image uses that API without a private board transport.
    let main_index = artifact
        .functions
        .iter()
        .position(|function| function.name == "main")
        .unwrap();
    artifact.functions.swap(0, main_index);
    objects.insert(0, artifact);
    let mut image =
        saltwater_sia::Artifact::link_reachable_images(&objects, 0x10000, "main", 0x60000)
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
                instruction_limit: budget,
                cycle_limit: 800_000_000,
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
        println!(
            "PRODUCTION-BOARD cycles={} backend_transactions={} minimum_sp=0x{:x}",
            result.cycles, result.backend_transactions, result.minimum_sp
        );
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
        for count in 1..=budget {
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
    println!("PASS SIA C software floating-point source {path}; {instructions} instructions; board={board}");
}
