use std::convert::TryInto;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use saltwater_sia::{compile, Opt, TARGET};

const AMD64: &str = "x86_64-unknown-linux-gnu";
const HELP: &str = "\
cosmicc - C compiler with SIA32 and amd64 targets

Usage: cosmicc [OPTIONS] [FILE ...]

The default SIA32 target emits a COSMIC-SIA bundle. `--emit-elf` writes a
linkable ELF32 SIA relocatable object (machine 0xff53, REL/ABS32 0x80). The
amd64 Linux target emits ELF64 objects with -c, or links through the system C
driver otherwise.

Options:
  -c, --no-link        Compile only (amd64 ELF object; SIA bundle by default).
      --emit-elf       Emit an ELF32 SIA object instead of a COSMIC-SIA bundle.
  -o, --output PATH    Output (default: a.sia, a.o, or a.out by target/mode).
  -I, --include DIR    Add a target header search directory.
  -D, --define DEF     Define NAME or NAME=VALUE.
  -O0/-O1/-O2/-Os     amd64 Cranelift none/speed/speed/speed_and_size.\n  -E, --preprocess     Preprocess one source to stdout.
      --target TARGET  sia32-unknown-none (default), x86_64-unknown-linux-gnu,
                       or amd64 as an alias for x86_64-unknown-linux-gnu.
  -l NAME             Add a library when linking amd64.
  -L DIR              Add a library directory when linking amd64.
  -h, --help           Show help.
  -V, --version        Show version.

No input, or -, reads standard input. Multiple C sources and existing .o/.a
files may be linked for amd64. CC selects the linker driver (default: cc).
Complete C compatibility remains in progress; see PORTABILITY_TODO.md.";

fn main() {
    let mut inputs = Vec::new();
    let mut output = None;
    let mut include_paths = Vec::new();
    let mut definitions = Vec::new();
    let mut link_args = Vec::new();
    let mut preprocess_only = false;
    let mut compile_only = false;
    let mut emit_elf = false;
    let mut target = TARGET;
    let mut optimization = saltwater_amd64::Optimization::None;
    let mut optimization_requested = false;
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("cosmicc {} (SIA32, amd64 Linux)", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-O0" | "-O1" | "-O2" | "-Os" => {
                optimization_requested = true;
                optimization = match argument.as_str() {
                    "-O0" => saltwater_amd64::Optimization::None,
                    "-Os" => saltwater_amd64::Optimization::SpeedAndSize,
                    _ => saltwater_amd64::Optimization::Speed,
                };
            }
            "-c" | "--no-link" => compile_only = true,
            "--emit-elf" => emit_elf = true,
            "-E" | "--preprocess" | "--preprocess-only" => preprocess_only = true,
            "-o" | "--output" => output = Some(PathBuf::from(next(&mut args, "output path"))),
            "-I" | "--include" => {
                include_paths.push(PathBuf::from(next(&mut args, "include directory")))
            }
            "-D" | "--define" => definitions.push(next(&mut args, "macro definition")),
            "-l" | "-L" => {
                link_args.push(argument.clone());
                link_args.push(next(&mut args, "link argument"));
            }
            "--target" => {
                let value = next(&mut args, "target");
                target = match value.as_str() {
                    TARGET => TARGET,
                    AMD64 | "amd64" => AMD64,
                    _ => usage_error(&format!(
                        "unsupported target `{value}`; expected `{TARGET}` or `{AMD64}`"
                    )),
                };
            }
            _ if argument.starts_with("-I") && argument.len() > 2 => {
                include_paths.push(PathBuf::from(&argument[2..]))
            }
            _ if argument.starts_with("-D") && argument.len() > 2 => {
                definitions.push(argument[2..].to_owned())
            }
            _ if (argument.starts_with("-l") || argument.starts_with("-L"))
                && argument.len() > 2 =>
            {
                link_args.push(argument)
            }
            _ if argument.starts_with('-') && argument != "-" => {
                usage_error(&format!("unknown option `{argument}`"))
            }
            _ => inputs.push(PathBuf::from(argument)),
        }
    }
    if optimization_requested && target == TARGET {
        usage_error("optimization options currently require amd64");
    }
    if emit_elf && target != TARGET {
        usage_error("--emit-elf currently applies to the SIA32 target");
    }
    if inputs.is_empty() {
        inputs.push(PathBuf::from("-"));
    }
    if inputs.len() != 1 && (target == TARGET || compile_only || preprocess_only) {
        usage_error("this target/mode requires exactly one source input");
    }
    if !link_args.is_empty() && (target == TARGET || compile_only || preprocess_only) {
        usage_error("-l/-L require amd64 linking mode");
    }
    if inputs.iter().filter(|p| p.as_os_str() == "-").count() > 1 {
        usage_error("standard input may occur only once");
    }
    let output = output.unwrap_or_else(|| {
        PathBuf::from(if target == TARGET && !emit_elf {
            "a.sia"
        } else if target == TARGET || compile_only {
            "a.o"
        } else {
            "a.out"
        })
    });
    let mut opt = Opt::default();
    opt.search_path = include_paths;
    if target == AMD64 {
        saltwater_amd64::configure_options(&mut opt);
    }
    for definition in definitions {
        let mut parts = definition.splitn(2, '=');
        let name = parts.next().unwrap();
        let value = parts
            .next()
            .unwrap_or("1")
            .try_into()
            .unwrap_or_else(|error: saltwater_sia::LexError| fatal(&error.to_string()));
        opt.definitions.insert(name.into(), value);
    }
    if preprocess_only {
        opt.filename = inputs[0].clone();
        let program = saltwater_sia::preprocess(&read_source(&inputs[0]), opt);
        match program.result {
            Ok(tokens) => {
                for token in tokens {
                    print!("{}", token.data);
                }
            }
            Err(errors) => fatal(
                &errors
                    .into_iter()
                    .map(|error| error.data.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        }
        return;
    }
    if target == TARGET {
        opt.filename = inputs[0].clone();
        let artifact = compile(&read_source(&inputs[0]), opt)
            .unwrap_or_else(|error| fatal(&error.to_string()));
        write_output(
            &output,
            &(if emit_elf {
                artifact.to_elf_bytes()
            } else {
                artifact.to_bytes()
            })
            .unwrap_or_else(|error| fatal(&error.to_string())),
        );
    } else if compile_only {
        opt.filename = inputs[0].clone();
        let bytes =
            saltwater_amd64::compile_with_optimization(&read_source(&inputs[0]), opt, optimization)
                .unwrap_or_else(|error| fatal(&error.to_string()));
        write_output(&output, &bytes);
    } else {
        let scratch = Scratch::new();
        let mut objects = Vec::new();
        for (index, input) in inputs.iter().enumerate() {
            if matches!(
                input.extension().and_then(|e| e.to_str()),
                Some("o" | "a" | "so")
            ) {
                objects.push(
                    fs::canonicalize(input)
                        .unwrap_or_else(|error| fatal(&format!("{}: {error}", input.display()))),
                );
            } else {
                let mut source_opt = opt.clone();
                source_opt.filename = input.clone();
                let bytes = saltwater_amd64::compile_with_optimization(
                    &read_source(input),
                    source_opt,
                    optimization,
                )
                .unwrap_or_else(|error| fatal(&error.to_string()));
                let object = scratch.0.join(format!("source-{index}.o"));
                write_output(&object, &bytes);
                objects.push(object);
            }
        }
        let driver = env::var_os("CC").unwrap_or_else(|| "cc".into());
        let status = Command::new(&driver)
            .args(&objects)
            .args(&link_args)
            .arg("-o")
            .arg(&output)
            .status()
            .unwrap_or_else(|error| {
                fatal(&format!(
                    "failed to run linker driver {:?}: {error}",
                    driver
                ))
            });
        if !status.success() {
            // Drop scratch before exit; fatal's process::exit does not run destructors.
            drop(scratch);
            fatal(&format!("linker driver exited with {status}"));
        }
    }
}

fn next(args: &mut impl Iterator<Item = String>, what: &str) -> String {
    args.next()
        .unwrap_or_else(|| usage_error(&format!("missing {what}")))
}
fn read_source(path: &Path) -> String {
    if path.as_os_str() == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .unwrap_or_else(|error| fatal(&format!("failed to read standard input: {error}")));
        source
    } else {
        fs::read_to_string(path)
            .unwrap_or_else(|error| fatal(&format!("failed to read {}: {error}", path.display())))
    }
}
fn write_output(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes)
        .unwrap_or_else(|error| fatal(&format!("failed to write {}: {error}", path.display())));
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("cosmicc-amd64-{}-{stamp}", process::id()));
        fs::create_dir(&path).unwrap_or_else(|error| {
            fatal(&format!("cannot create linker scratch directory: {error}"))
        });
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn usage_error(message: &str) -> ! {
    eprintln!("cosmicc: {message}\nTry `cosmicc --help`.");
    process::exit(1);
}
fn fatal(message: &str) -> ! {
    eprintln!("cosmicc: error: {message}");
    process::exit(2);
}
