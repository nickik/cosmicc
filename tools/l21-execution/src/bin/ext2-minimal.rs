use lighting_simulation::isa::bus::PhysicalBus;
use lighting_simulation::isa::machine::LightingMachine;
use lighting_simulation::isa::rax_qdx_b_physical_device::{
    RaxPhysicalQdxBDevice, RaxQdxBFileDisk, RaxQdxBRamDisk,
};

fn main() {
    let image = std::env::args()
        .nth(1)
        .expect("usage: ext2-minimal DISPOSABLE.img");
    let card = RaxPhysicalQdxBDevice::with_backends(
        Box::new(RaxQdxBRamDisk::new(64, 512).unwrap()),
        Box::new(RaxQdxBFileDisk::open(&image, 1024, false).expect("open disposable ext2 image")),
    )
    .unwrap();
    let artifact = saltwater_sia::compile_default(include_str!("../../../ext2/minimal-write.c"))
        .expect("compile minimal ext2 writer");
    let mut plan = artifact
        .prepare_integer_call("minimal_write", 0x10000, &[])
        .unwrap();
    plan.registers[13] = 0xf0000;
    let mut machine = LightingMachine::new(1024 * 1024, &[]).unwrap();
    machine.load_ram(plan.entry_address, &plan.code).unwrap();
    machine.attach_plio0_device(0, Box::new(card)).unwrap();
    machine
        .cpu_mut()
        .core_mut()
        .set_pc_for_test(plan.entry_address);
    for (reg, value) in plan.registers.into_iter().enumerate() {
        machine.cpu_mut().core_mut().write_reg(reg, value);
    }
    for count in 1..=100000 {
        machine.step_detailed().expect("Lighting SIA32/QDX-B step");
        if machine.cpu().privileged_state().cause != 0 {
            let state = *machine.cpu().privileged_state();
            panic!(
                "architectural trap: {:?}; MMIO retry={:?}; host error={:?}",
                state,
                machine.bus_mut().read16(state.badaddr),
                machine.physical_read32(0xffe00028)
            );
        }
        if machine.cpu().core().pc() == plan.return_address {
            let status = machine.cpu().core().read_reg(1);
            assert_eq!(
                status,
                0,
                "guest ext2/QDX-B error; host={:?} sq={:?} cq={:?}",
                machine.physical_read32(0xffe00028),
                machine.physical_read32(0x5000),
                machine.physical_read32(0x600c)
            );
            assert_eq!(machine.cpu().core().read_reg(13), 0xf0000, "stack restored");
            println!("PASS native SIA32 ext2 overwrite/readback through physical QDX-B: {count} instructions");
            return;
        }
    }
    panic!(
        "guest instruction budget exceeded: {}",
        machine.register_dump()
    );
}
