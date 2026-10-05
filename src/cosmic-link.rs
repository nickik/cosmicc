//! Static multi-bundle image builder. It emits raw SIA RAM bytes, not an OS
//! executable or a host binary. Address/entry metadata is printed explicitly.
use saltwater_sia::Artifact;
use std::{env, fs, process};

fn fatal(message: impl std::fmt::Display) -> ! {
    eprintln!("cosmic-link: {message}");
    process::exit(2)
}
fn number(value: &str) -> u32 {
    let result = if let Some(hex) = value.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)
    } else {
        value.parse()
    };
    result.unwrap_or_else(|_| fatal(format!("invalid SIA32 address/size `{value}`")))
}
fn main() {
    let mut args = env::args().skip(1);
    let mut inputs = Vec::new();
    let mut output = None;
    let mut base = None;
    let mut entry = None;
    let mut max_bytes = None;
    let mut container = false;
    let mut gc_sections = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Usage: cosmic-link --base ADDRESS --entry SYMBOL --max-bytes SIZE -o IMAGE.bin BUNDLE.sia... [--container] [--gc-sections]\n\nResolves Abs4 relocations across COSMIC-SIA v4 bundles. Numbers are decimal\nor 0x-prefixed hexadecimal. --gc-sections retains the entry relocation closure. Default output is raw RAM bytes; --container adds versioned image metadata.\nLoader protection,\ndynamic imports, archives and ELF inputs are unsupported.");
                return;
            }
            "--container" => container = true,
            "--gc-sections" => gc_sections = true,
            "--base" => {
                base = Some(number(
                    &args.next().unwrap_or_else(|| fatal("missing --base value")),
                ))
            }
            "--max-bytes" => {
                max_bytes = Some(number(
                    &args
                        .next()
                        .unwrap_or_else(|| fatal("missing --max-bytes value")),
                ))
            }
            "--entry" => {
                entry = Some(
                    args.next()
                        .unwrap_or_else(|| fatal("missing --entry symbol")),
                )
            }
            "-o" | "--output" => {
                output = Some(args.next().unwrap_or_else(|| fatal("missing output path")))
            }
            _ if arg.starts_with('-') => fatal(format!("unknown option `{arg}`")),
            _ => {
                inputs.push(arg);
            }
        }
    }
    if inputs.is_empty() {
        fatal("missing input bundles");
    }
    let output = output.unwrap_or_else(|| fatal("missing -o output path"));
    if std::path::Path::new(&output).exists() {
        fatal("output exists; choose a new path");
    }
    let artifacts = inputs
        .iter()
        .map(|input| {
            let bytes = fs::read(input).unwrap_or_else(|error| fatal(error));
            Artifact::from_bytes(&bytes).unwrap_or_else(|error| fatal(error))
        })
        .collect::<Vec<_>>();
    let link = if gc_sections {
        Artifact::link_reachable_images
    } else {
        Artifact::link_images
    };
    let image = link(
        &artifacts,
        base.unwrap_or_else(|| fatal("missing --base")),
        &entry.unwrap_or_else(|| fatal("missing --entry")),
        max_bytes.unwrap_or_else(|| fatal("missing --max-bytes")),
    )
    .unwrap_or_else(|error| fatal(error));
    // create_new also prevents a racing writer from being overwritten.
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap_or_else(|error| fatal(error));
    let encoded = if container {
        image.to_image_bytes().unwrap_or_else(|error| fatal(error))
    } else {
        image.bytes.clone()
    };
    file.write_all(&encoded)
        .unwrap_or_else(|error| fatal(error));
    println!(
        "base=0x{:08x} entry=0x{:08x} bytes={} output={}",
        image.base,
        image.entry,
        image.bytes.len(),
        output
    );
    for region in &image.regions {
        println!(
            "0x{:08x} size={} {} {}",
            region.address,
            region.size,
            if region.executable {
                "code"
            } else if region.read_only {
                "rodata"
            } else {
                "data"
            },
            region.name
        );
    }
}
