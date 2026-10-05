//! Execute previously compiled, no-argument/no-output compatibility fixtures.
//! No guest calls are serviced by host C code. Link failures remain failures.
use lighting_simulation::isa::machine::LightingMachine;
use saltwater_sia::Artifact;
use std::path::Path;

fn execute(path: &Path, runtime: &[Artifact]) -> Result<(String, u64), String> {
    let artifact = Artifact::from_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("bundle-error:{e}"))?;
    let mut objects = vec![artifact];
    objects.extend_from_slice(runtime);
    let image = Artifact::link_images(&objects, 0x10000, "main", 0x60000)
        .map_err(|e| format!("link-failure:{e}"))?;
    let stack = 0xf0000;
    let return_address = 0x80000;
    let mut machine = LightingMachine::new(1024 * 1024, &[]).map_err(|e| format!("machine:{e}"))?;
    machine
        .load_ram(image.base, &image.bytes)
        .map_err(|e| format!("load:{e}"))?;
    machine.cpu_mut().core_mut().set_pc_for_test(image.entry);
    machine.cpu_mut().core_mut().write_reg(13, stack);
    machine.cpu_mut().core_mut().write_reg(14, return_address);
    for count in 1..=20_000_000 {
        let pc = machine.cpu().core().pc();
        if !image
            .regions
            .iter()
            .any(|r| r.executable && pc >= r.address && pc < r.address + r.size)
        {
            return Ok((format!("pc-outside-code:0x{pc:08x}"), count - 1));
        }
        machine
            .step_detailed()
            .map_err(|e| format!("execution-error:{e}"))?;
        let cause = machine.cpu().privileged_state().cause;
        if cause != 0 {
            return Ok((format!("architectural-trap:{cause}"), count));
        }
        if machine.cpu().core().pc() == return_address {
            let value = machine.cpu().core().read_reg(1);
            if machine.cpu().core().read_reg(13) != stack {
                return Ok(("stack-not-restored".into(), count));
            }
            return Ok((
                if value == 0 {
                    "pass".into()
                } else {
                    format!("nonzero-return:{value}")
                },
                count,
            ));
        }
    }
    Ok(("instruction-budget".into(), 20_000_000))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let directory = args.next().expect("compiled inventory directory");
    // Runtime objects are explicit inputs and execute guest instructions only.
    let runtime: Vec<_> = args
        .map(|path| {
            let bytes = std::fs::read(&path).expect("read runtime object");
            Artifact::from_bytes(&bytes).expect("decode runtime object")
        })
        .collect();
    let directory = Path::new(&directory);
    let candidates = std::fs::read_to_string(directory.join("execution-candidates.txt"))
        .expect("generated execution candidate list");
    println!("case\tstatus\tinstructions\tmachine");
    for case in candidates.lines().filter(|s| !s.is_empty()) {
        let result = std::panic::catch_unwind(|| execute(&directory.join(case), &runtime));
        let (status, instructions) = match result {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => (error.replace(['\t', '\n', '\r'], " "), 0),
            Err(_) => ("runner-panic".into(), 0),
        };
        println!("{case}\t{status}\t{instructions}\tLightingMachine");
    }
}
