//! Focused frontend regressions executed by Lighting guest instructions.
use saltwater_sia::compile_default;
fn source() -> String {
    r#"#define DO2(x) x; x
#define DO4(x) DO2(x); DO2(x)
#define DO8(x) DO4(x); DO4(x)
int a[] = {[4] = 9, [1] = 2};
int calls;
int values[3] = {1,2,3};
int index_values[256];
int indexed(unsigned char position) { return index_values[position]; }
int indexed_signed(signed char position) { return (&index_values[128])[position]; }
int *pick(void) { return &values[calls++]; }
struct Compound { int a; long b; };
struct Compound *compound_global = &(struct Compound){.a=11, .b=12L};
struct Anonymous { char tag; union {long value; int alternative;}; struct {int other;}; };
struct Anonymous anonymous_global = {1, 2L, 3};
typedef int Shadow;
int array_parameter(int values[static 4]) {return values[3];}
#define CAT(a,b) a ## b
#define XCAT(a,b) CAT(a,b)
#define LEFT foo
#define RIGHT bar
#define NEXT_LINE 1000
#line NEXT_LINE
#if __LINE__ != 1000
#error macro-expanded line directive failed
#endif
int main(void) {
 char boolean_size_operand;
 if (sizeof(!boolean_size_operand)!=sizeof(int) || sizeof(boolean_size_operand&&boolean_size_operand)!=sizeof(int) || sizeof(boolean_size_operand==boolean_size_operand)!=sizeof(int)) return 26;
 unsigned int shift_value=0x80000000U; long long shift_count=1;
 if (sizeof(shift_value>>shift_count)!=sizeof(unsigned int) || (shift_value>>shift_count)!=0x40000000U) return 27;
 int b[][2] = {1,2,3,4}; char s[] = "abc";
 int i = 0;
 if (sizeof(a) + sizeof(b) + sizeof(s) != 40) return 1;
 while (i < 3) { *pick() += 4; i++; }
 if (calls != 3 || values[0] != 5 || values[1] != 6 || values[2] != 7) return 2;
 if (0 && (*pick() += 10)) return 3;
 if (calls != 3) return 4;
 i = 0;
 while ((values[0] -= 1) > 0) { i++; }
 if (i != 4 || values[0] != 0) return 5;
 for (; (values[1] -= 1) > 0; values[2] += 2) {}
 if (values[1] != 0 || values[2] != 17) return 6;
 { int local[3] = {1,2,3}; int *p = &local[2]; const int *q;
   --p; if (*p != 2 || p-local != 1) return 7;
   q = p; if (q-local != 1 || q < p) return 8;
   q = i ? q : p; if (*q != 2) return 9;
 }
 i = 0; DO8(i++); if (i != 8) return 10;
 { char c = 7; short s = 300; char *p = &c;
   if (*p != 7 || s != 300) return 11; }
 for (i = 0; i < 256; i++) index_values[i] = i * 37 + 11;
 for (i = 0; i < 256; i++)
   if (indexed((unsigned char)i) != i * 37 + 11) return 12;
 if (indexed_signed(-128) != 11 || indexed_signed(127) != 255 * 37 + 11) return 13;
 {int counter=0; struct Compound *p=&(struct Compound){counter++,7L};
  int *a=(int[]){3,4,5}; int *scalar=&(int){6}; int foobar=9;
  if(p->a || p->b!=7L || counter!=1 || a[2]!=5) return 14;
  *scalar+=2; if(*scalar!=8) return 21; if(XCAT(LEFT,RIGHT)!=9) return 22;
  if(((struct Compound){.b=10L}).a!=0 || ((struct Compound){9,10L}).b!=10L) return 16;
  if(0 && (int){counter++}) return 17;
  for(i=0;i<3;i++) {p=&(struct Compound){counter++,i}; if(p->b!=i) return 18;}
  if(counter!=4 || compound_global->a!=11 || compound_global->b!=12L) return 19;
 }
 anonymous_global.value++; anonymous_global.other+=2;
 if(anonymous_global.tag!=1 || anonymous_global.value!=3 || anonymous_global.other!=5) return 20;
 {int Shadow=7; int raw[4]={1,2,3,4};
  {typedef long Shadow; Shadow value=9L; if(value!=9L)return 23;}
  if(Shadow!=7 || array_parameter(raw)!=4) return 24;
 }
 if(sizeof(L'\0')!=4 || L'Ω'!=937 || L'\u03a9'!=937 || L'\x1234'!=4660) return 25;
 return 0;
}"#
    .to_owned()
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
    println!("PASS frontend arrays, compound assignments, nested macros, pointer arithmetic and narrow locals; {instructions} instructions; board={board}");
}
