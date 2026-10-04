use lighting_simulation::native_board_call::{run_native_board_call, NativeBoardCall};

fn main() {
    let bridge = std::env::var("LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE")
        .expect("set LIGHTING_MAINBOARD_M6_MULTISLOT_BRIDGE to the real Bluesim bridge");
    // e2fsprogs ext2_types.h selects unsigned long long for its __u64.
    let artifact = saltwater_sia::compile_default(
        "typedef unsigned long long __u64; unsigned probe(void) { return sizeof(__u64)*100 + sizeof(long)*10 + sizeof(void*); }",
    ).expect("compile ext2 ABI probe");
    let mut plan = artifact.prepare_integer_call("probe", 0x8000, &[]).unwrap();
    plan.registers[13] = 0xf0000;
    let result = run_native_board_call(
        &bridge,
        &NativeBoardCall {
            code: &plan.code,
            entry: plan.entry_address,
            return_address: plan.return_address,
            registers: plan.registers,
            instruction_limit: 1000,
            cycle_limit: 10000,
        },
    )
    .expect("execute ext2 ABI probe on native SIA32/mainboard path");
    let widths = result.state.registers[1];
    println!(
        "ext2 native ABI: __u64={} long={} pointer={}",
        widths / 100,
        (widths / 10) % 10,
        widths % 10
    );
    if widths != 844 {
        eprintln!("ext2 ABI rejected: requires 8-byte __u64 and 4-byte long/pointer; filesystem execution is unsafe with this data model");
        std::process::exit(2);
    }
}
