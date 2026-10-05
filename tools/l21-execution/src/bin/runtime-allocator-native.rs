//! Guest-only freestanding arena/string acceptance on Lighting.
use saltwater_sia::compile_default;
fn source() -> String {
    let mut source = include_str!("../../../../runtime/include/cosmic_runtime_v1.h").to_owned();
    source.push_str(
        &include_str!("../../../../runtime/allocator.c")
            .replace("#include \"include/cosmic_runtime_v1.h\"", ""),
    );
    source.push_str(
        &include_str!("../../../../runtime/strings.c")
            .replace("#include \"include/cosmic_runtime_v1.h\"", ""),
    );
    source.push_str(
        r#"
unsigned char storage[129];
unsigned char stress_storage[257];
int arena_stress(void) {
    cosmic_arena a;
    unsigned char *slots[8];
    unsigned int sizes[8];
    unsigned int step, index, i, j, k, n;
    unsigned char *p;
    if (cosmic_arena_init(&a, stress_storage+1, 256)) return 1;
    for (i = 0; i < 8; i++) { slots[i] = 0; sizes[i] = 0; }
    for (step = 0; step < 96; step++) {
        index = step % 8;
        n = (step * 13u) % 33u + 1u;
        if (slots[index] == 0) {
            p = cosmic_arena_alloc(&a, n);
            if (p != 0) {
                slots[index] = p; sizes[index] = n;
                for(i = 0; i < n; i++) p[i] = (unsigned char)(index*17u+i);
            }
        } else if (step % 3u == 0) {
            if (cosmic_arena_free(&a, slots[index])) return 2;
            slots[index] = 0; sizes[index] = 0;
        } else {
            p = cosmic_arena_realloc(&a, slots[index], n);
            if (p != 0) {
                k = sizes[index] < n ? sizes[index] : n;
                for(i = 0; i < k; i++) if(p[i] != index*17u+i) return 3;
                slots[index] = p; sizes[index] = n;
                for(i = k; i < n; i++) p[i] = (unsigned char)(index*17u+i);
            }
        }
        for(j = 0; j < 8; j++) if(slots[j] != 0) {
            if (((unsigned)slots[j] & 7u) != 0) return 4;
            for(i = 0; i < sizes[j]; i++) if(slots[j][i] != j*17u+i) return 5;
            for(k = j+1; k < 8; k++) if(slots[k] != 0)
                if ((unsigned)slots[j] < (unsigned)slots[k]+sizes[k]
                    && (unsigned)slots[k] < (unsigned)slots[j]+sizes[j]) return 6;
        }
    }
    for(i = 0; i < 8; i++) if(cosmic_arena_free(&a, slots[i])) return 7;
    if(a.used != 0) return 8;
    return 0;
}
int main(void) {
    cosmic_arena a;
    unsigned char *p, *q, *r;
    unsigned used, i;
    char text[5] = {'a','b','a',0,0};
    char *end;
    unsigned int status;
    char high[2] = { (char)255, 0 };
    if (cosmic_arena_init(&a, storage + 1, 128)) return 1;
    if (((unsigned)a.base & 7u) != 0) return 2;
    if (cosmic_arena_alloc(&a, 0) != 0 || a.used != 0) return 3;
    p = cosmic_arena_alloc(&a, 9);
    if (p == 0 || ((unsigned)p & 7u) != 0) return 4;
    for (i = 0; i < 9; i++) p[i] = (unsigned char)(i + 10);
    q = cosmic_arena_alloc(&a, 1);
    if (q == 0 || q == p) return 5;
    q[0] = 99;
    r = cosmic_arena_realloc(&a, p, 17);
    if (r == 0 || r == p || q[0] != 99) return 6;
    for (i = 0; i < 9; i++) if (r[i] != i + 10) return 7;
    used = a.used;
    if (cosmic_arena_realloc(&a, r, 0xffffffffu) != 0 || a.used != used) return 8;
    for (i = 0; i < 9; i++) if (r[i] != i + 10) return 9;
    if (cosmic_arena_realloc(&a, r + 1, 20) != 0 || a.used != used) return 10;
    /* Zero realloc is tested after shrink below. */
    if (cosmic_arena_realloc(&a, r, 4) != r || a.used != used) return 12;
    if (cosmic_arena_alloc(&a, 128) != 0 || a.used != used) return 13;
    if (cosmic_arena_free(&a, p) == 0) return 25;
    if (cosmic_arena_free(&a, q+1) == 0 || cosmic_arena_free(&a, 0) != 0) return 26;
    if (cosmic_arena_free(&a, q) != 0) return 27;
    if (cosmic_arena_realloc(&a, r, 0) != 0 || a.used != 0) return 28;
    if (cosmic_arena_free(&a, r) == 0) return 29;
    p = cosmic_arena_alloc(&a, 8);
    q = cosmic_arena_alloc(&a, 8);
    r = cosmic_arena_alloc(&a, 8);
    used = a.used;
    if (cosmic_arena_free(&a, p) || cosmic_arena_free(&a, q)) return 30;
    p = cosmic_arena_alloc(&a, 24);
    if (p != a.base+8 || a.used != used) return 31;
    if (cosmic_arena_free(&a, p)) return 32;
    q = cosmic_arena_alloc(&a, 1);
    if (q != a.base+8 || a.used != used) return 33;
    if (cosmic_arena_free(&a, q) || cosmic_arena_free(&a, r) || a.used != 0) return 34;
    p = cosmic_arena_calloc(&a, 4, 3);
    if (p == 0) return 35;
    for(i = 0; i < 12; i++) if(p[i] != 0) return 36;
    used = a.used;
    if (cosmic_arena_calloc(&a, 0x80000000u, 2) != 0 || a.used != used) return 37;
    cosmic_arena_reset(&a);
    if (a.used != 0 || cosmic_arena_realloc(&a, p, 1) != 0) return 14;
    if (cosmic_arena_realloc(&a, 0, 8) == 0) return 15;
    if (cosmic_strlen(text) != 3 || cosmic_strnlen(text,2) != 2) return 16;
    if (cosmic_strnlen(text,0) != 0 || cosmic_strnlen(text,8) != 3) return 17;
    if (cosmic_strcmp(text,text) != 0 || cosmic_strcmp(high,text) <= 0) return 18;
    if (cosmic_strncmp(text,high,0) != 0 || cosmic_strncmp(text,high,1) >= 0) return 19;
    if (cosmic_strchr(text,'a') != text || cosmic_strrchr(text,'a') != text+2) return 20;
    if (cosmic_strchr(text,0) != text+3 || cosmic_strrchr(text,0) != text+3) return 21;
    if (cosmic_strchr(text,'z') != 0 || cosmic_strrchr(text,'z') != 0) return 22;
    if (cosmic_arena_init(&a, 0, 8) == 0 || a.used != 0 || a.capacity != 0) return 23;
    if (cosmic_arena_init(&a, (void *)0xfffffff8u, 16) == 0) return 24;
    if (cosmic_strtoul32("  -1!", &end, 0, &status) != 0xffffffffu || status || *end != '!') return 40;
    if (cosmic_strtoul32("4294967295", &end, 10, &status) != 0xffffffffu || status || *end) return 41;
    if (cosmic_strtoul32("4294967296zzz", &end, 10, &status) != 0xffffffffu || status != 2 || *end != 'z') return 42;
    if (cosmic_strtoul32("0x10!", &end, 0, &status) != 16 || status || *end != '!') return 43;
    if (cosmic_strtoul32("0779", &end, 0, &status) != 63 || status || *end != '9') return 44;
    if (cosmic_strtoul32("0x", &end, 0, &status) != 0 || status || *end != 'x') return 45;
    if (cosmic_strtoul32(" +?", &end, 10, &status) != 0 || status != 1 || *end != ' ') return 46;
    if (cosmic_strtoul32("zZ", &end, 36, &status) != 1295 || status || *end) return 47;
    if (cosmic_strtoul32("123", &end, 1, &status) != 0 || status != 3 || *end != '1') return 48;
    if (cosmic_strtoul32("-4294967296", &end, 10, &status) != 0xffffffffu || status != 2 || *end) return 49;
    if (cosmic_strtoul32("10", 0, 2, 0) != 2) return 50;
    i = arena_stress();
    if (i != 0) return 100+i;
    return 0;
}
"#,
    );
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
    if std::env::var_os("COSMIC_RUNTIME_TRACE").is_some() {
        eprintln!("regions={:?}", image.regions);
    }
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
    println!("PASS arena allocation/reallocation and strings; {instructions} instructions; board={board}");
}
