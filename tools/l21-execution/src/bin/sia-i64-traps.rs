//! Implementation-policy tests for undefined C division inputs, not C conformance.
use lighting_simulation::isa::machine::LightingMachine;
fn main() {
    let cases = [
        (
            "zero-divisor",
            "unsigned long long n=123,d=0;int main(void){return (int)(n/d);}",
        ),
        (
            "signed-overflow",
            "long long n=(-9223372036854775807LL-1),d=-1;int main(void){return (int)(n/d);}",
        ),
    ];
    for (name, source) in cases {
        let artifact = saltwater_sia::compile_default(source).expect("compile trap policy fixture");
        let image = artifact
            .link_image(0x10000, "main", 0x40000)
            .expect("link fixture");
        let mut m = LightingMachine::new(1024 * 1024, &[]).unwrap();
        m.load_ram(image.base, &image.bytes).unwrap();
        m.cpu_mut().core_mut().set_pc_for_test(image.entry);
        m.cpu_mut().core_mut().write_reg(13, 0xf0000);
        m.cpu_mut().core_mut().write_reg(14, 0x80000);
        let mut trapped = false;
        for count in 1..=20_000 {
            let step = m.step_detailed();
            let cause = m.cpu().privileged_state().cause;
            if cause != 0 {
                assert_eq!(
                    cause,
                    u32::from(lighting_simulation::isa::privilege::CAUSE_TRAP)
                );
                println!("PASS {name}: architectural trap cause={cause}, instructions={count}, step_error={}",step.is_err());
                trapped = true;
                break;
            }
            step.expect("unexpected host execution error");
            assert_ne!(
                m.cpu().core().pc(),
                0x80000,
                "undefined division returned normally"
            );
        }
        assert!(trapped, "trap budget exceeded for {name}");
    }
}
