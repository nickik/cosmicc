//! Minimal, strict SIA ELF32 relocatable object writer/reader.
//!
//! This deliberately implements only the ABI used by Cosmic C and the SIA
//! linker: ELF32 little-endian ET_REL, machine 0xff53, and REL/ABS32 (0x80).
//! It does not pretend that host ELF linkers understand this private machine.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::convert::{TryFrom, TryInto};

use crate::{
    Artifact, DataArtifact, Error, FunctionArtifact, ImageRegion, LinkedImage, RelocationArtifact,
};

const EM_SIA32: u16 = 0xff53;
const R_SIA32_ABS32: u32 = 0x80;
const SHT_NULL: u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_REL: u32 = 9;
const SHT_NOBITS: u32 = 8;
const SHF_WRITE: u32 = 1;
const SHF_ALLOC: u32 = 2;
const SHF_EXECINSTR: u32 = 4;
const STB_LOCAL: u8 = 0;
const STB_GLOBAL: u8 = 1;
const STT_NOTYPE: u8 = 0;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;
const ET_EXEC: u16 = 2;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const MAX_ELF_OBJECT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
struct Section {
    name: String,
    kind: u32,
    flags: u32,
    align: u32,
    data: Vec<u8>,
    link: u32,
    info: u32,
    entsize: u32,
    name_offset: u32,
    file_offset: u32,
}

#[derive(Clone)]
struct Symbol {
    name: String,
    value: u32,
    size: u32,
    bind: u8,
    kind: u8,
    section: u16,
}

fn fail(message: impl Into<String>) -> Error {
    Error::Codegen(format!("SIA ELF: {}", message.into()))
}

fn as_u32(value: usize, what: &str) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| fail(format!("{what} exceeds ELF32 range")))
}

fn align_to(value: usize, alignment: usize) -> Result<usize, Error> {
    let alignment = alignment.max(1);
    if !alignment.is_power_of_two() {
        return Err(fail("section alignment is not a power of two"));
    }
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| fail("file layout overflow"))
}

fn string_offset(strings: &mut Vec<u8>, value: &str) -> Result<u32, Error> {
    if value.as_bytes().contains(&0) {
        return Err(fail("symbol or section name contains NUL"));
    }
    let offset = as_u32(strings.len(), "string table")?;
    strings.extend_from_slice(value.as_bytes());
    strings.push(0);
    Ok(offset)
}

fn append_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_symbol(out: &mut Vec<u8>, symbol: &Symbol, name_offset: u32) {
    append_u32(out, name_offset);
    append_u32(out, symbol.value);
    append_u32(out, symbol.size);
    out.push((symbol.bind << 4) | (symbol.kind & 0xf));
    out.push(0);
    append_u16(out, symbol.section);
}

impl LinkedImage {
    /// Encode this linked image as a static loadable SIA ELF32 ET_EXEC.
    pub fn to_elf_executable_bytes(&self) -> Result<Vec<u8>, Error> {
        write_executable(self)
    }

    /// Load a static SIA ELF32 ET_EXEC through the checked image model.
    pub fn from_elf_executable_bytes(bytes: &[u8], max_bytes: u32) -> Result<Self, Error> {
        read_executable(bytes, max_bytes)
    }
}

/// Encode a statically linked image as a loadable ELF32 SIA executable.
/// Program headers are the loader contract; zero-fill regions have p_filesz=0.
pub(super) fn write_executable(image: &LinkedImage) -> Result<Vec<u8>, Error> {
    image.validate_image(u32::MAX)?;
    let regions: Vec<_> = image
        .regions
        .iter()
        .filter(|region| region.size != 0)
        .collect();
    let phnum = u16::try_from(regions.len()).map_err(|_| fail("too many load segments"))?;
    let header_size = 52usize
        .checked_add(
            regions
                .len()
                .checked_mul(32)
                .ok_or_else(|| fail("program header size overflow"))?,
        )
        .ok_or_else(|| fail("program header size overflow"))?;
    let mut out = vec![0; header_size];
    out[0..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    out[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
    out[18..20].copy_from_slice(&EM_SIA32.to_le_bytes());
    out[20..24].copy_from_slice(&1u32.to_le_bytes());
    out[24..28].copy_from_slice(&image.entry.to_le_bytes());
    out[28..32].copy_from_slice(&52u32.to_le_bytes());
    out[40..42].copy_from_slice(&52u16.to_le_bytes());
    out[42..44].copy_from_slice(&32u16.to_le_bytes());
    out[44..46].copy_from_slice(&phnum.to_le_bytes());

    for (index, region) in regions.iter().enumerate() {
        let start = region
            .address
            .checked_sub(image.base)
            .ok_or_else(|| fail("load segment precedes image base"))? as usize;
        let end = start
            .checked_add(region.size as usize)
            .ok_or_else(|| fail("load segment range overflow"))?;
        let contents = image
            .bytes
            .get(start..end)
            .ok_or_else(|| fail("load segment exceeds linked image bytes"))?;
        let file_size = if region.zero_fill { 0 } else { region.size };
        let file_offset = as_u32(out.len(), "load segment file offset")?;
        if file_size != 0 {
            out.extend_from_slice(contents);
        }
        let flags = PF_R
            | if region.read_only { 0 } else { PF_W }
            | if region.executable { PF_X } else { 0 };
        let at = 52 + index * 32;
        for (offset, value) in [
            (0, PT_LOAD),
            (4, file_offset),
            (8, region.address),
            (12, region.address),
            (16, file_size),
            (20, region.size),
            (24, flags),
            (28, 1),
        ] {
            out[at + offset..at + offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(out)
}

/// Load a static SIA ET_EXEC into the same checked flat-memory model used by
/// CSIAIMG startup. Only PT_LOAD segments are accepted; there is no dynamic ABI.
pub(super) fn read_executable(bytes: &[u8], max_bytes: u32) -> Result<LinkedImage, Error> {
    if slice(bytes, 0, 7, "ELF executable identity")? != b"\x7fELF\x01\x01\x01"
        || u16_at(bytes, 16, "ELF executable type")? != ET_EXEC
        || u16_at(bytes, 18, "ELF executable machine")? != EM_SIA32
        || u32_at(bytes, 20, "ELF executable version")? != 1
        || u16_at(bytes, 40, "ELF executable header size")? != 52
        || u32_at(bytes, 32, "section header offset")? != 0
        || u16_at(bytes, 46, "section header size")? != 0
        || u16_at(bytes, 48, "section count")? != 0
        || u16_at(bytes, 50, "section name table index")? != 0
    {
        return Err(fail("expected static ELF32 little-endian SIA ET_EXEC"));
    }
    let entry = u32_at(bytes, 24, "ELF entry")?;
    let phoff = u32_at(bytes, 28, "program header offset")? as usize;
    let phentsize = u16_at(bytes, 42, "program header entry size")? as usize;
    let phnum = u16_at(bytes, 44, "program header count")? as usize;
    if phoff != 52 || phentsize != 32 || phnum == 0 {
        return Err(fail("invalid ELF program header table"));
    }
    slice(
        bytes,
        phoff,
        phnum
            .checked_mul(32)
            .ok_or_else(|| fail("program header size overflow"))?,
        "program header table",
    )?;
    let mut segments = Vec::with_capacity(phnum);
    let table_end = phoff + phnum * 32;
    let mut file_spans = Vec::<(usize, usize)>::new();
    let mut base = u32::MAX;
    let mut end = 0u64;
    for index in 0..phnum {
        let at = phoff + index * 32;
        let kind = u32_at(bytes, at, "program header type")?;
        let offset = u32_at(bytes, at + 4, "segment file offset")? as usize;
        let address = u32_at(bytes, at + 8, "segment virtual address")?;
        let physical = u32_at(bytes, at + 12, "segment physical address")?;
        let file_size = u32_at(bytes, at + 16, "segment file size")?;
        let memory_size = u32_at(bytes, at + 20, "segment memory size")?;
        let flags = u32_at(bytes, at + 24, "segment flags")?;
        let alignment = u32_at(bytes, at + 28, "segment alignment")?;
        if kind != PT_LOAD
            || physical != address
            || file_size > memory_size
            || memory_size == 0
            || flags & PF_R == 0
            || flags & !(PF_R | PF_W | PF_X) != 0
            || flags & PF_W != 0 && flags & PF_X != 0
            || alignment != 1
        {
            return Err(fail("unsupported or invalid PT_LOAD segment"));
        }
        if file_size != 0 && offset < table_end {
            return Err(fail("PT_LOAD file contents overlap ELF headers"));
        }
        slice(bytes, offset, file_size as usize, "segment contents")?;
        let file_end = offset
            .checked_add(file_size as usize)
            .ok_or_else(|| fail("segment file range overflow"))?;
        if file_size != 0
            && file_spans
                .iter()
                .any(|&(start, end)| offset < end && file_end > start)
        {
            return Err(fail("overlapping PT_LOAD file segments"));
        }
        if file_size != 0 {
            file_spans.push((offset, file_end));
        }
        let segment_end = u64::from(address) + u64::from(memory_size);
        if segment_end > u64::from(u32::MAX) + 1 {
            return Err(fail("segment address overflow"));
        }
        base = base.min(address);
        end = end.max(segment_end);
        segments.push((offset, address, file_size, memory_size, flags));
    }
    let length = end
        .checked_sub(u64::from(base))
        .ok_or_else(|| fail("invalid load address span"))?;
    if length == 0 || length > u64::from(max_bytes) {
        return Err(fail("ELF image exceeds loader budget"));
    }
    let mut image_bytes =
        vec![0; usize::try_from(length).map_err(|_| fail("image size overflow"))?];
    let mut regions = Vec::with_capacity(phnum);
    let mut spans = Vec::<(u64, u64)>::new();
    let mut entry_valid = false;
    for (index, (offset, address, file_size, memory_size, flags)) in
        segments.into_iter().enumerate()
    {
        let segment_end = u64::from(address) + u64::from(memory_size);
        if spans
            .iter()
            .any(|&(start, end)| u64::from(address) < end && segment_end > start)
        {
            return Err(fail("overlapping PT_LOAD memory segments"));
        }
        spans.push((u64::from(address), segment_end));
        let start = (address - base) as usize;
        let contents = slice(bytes, offset, file_size as usize, "segment contents")?;
        image_bytes[start..start + contents.len()].copy_from_slice(contents);
        let executable = flags & PF_X != 0;
        let read_only = flags & PF_W == 0;
        if executable && entry >= address && u64::from(entry) < segment_end {
            entry_valid = true;
        }
        regions.push(ImageRegion {
            name: format!("LOAD{index}"),
            address,
            size: memory_size,
            read_only,
            executable,
            zero_fill: file_size == 0 && !read_only && !executable,
        });
    }
    if !entry_valid {
        return Err(fail("ELF entry is outside executable PT_LOAD segments"));
    }
    let image = LinkedImage {
        base,
        entry,
        bytes: image_bytes,
        symbols: Default::default(),
        regions,
    };
    image.validate_image(max_bytes)?;
    Ok(image)
}

/// Write the canonical per-function/per-object SIA ELF layout.
pub(super) fn write(artifact: &Artifact) -> Result<Vec<u8>, Error> {
    artifact.validate()?;
    let locals = artifact.local_symbols.as_ref().cloned().unwrap_or_default();
    let mut sections = vec![Section {
        name: String::new(),
        kind: SHT_NULL,
        flags: 0,
        align: 0,
        data: Vec::new(),
        link: 0,
        info: 0,
        entsize: 0,
        name_offset: 0,
        file_offset: 0,
    }];
    let mut definitions = HashMap::<String, (u16, u32, u32, u8)>::new();
    let mut pending_relocations = Vec::<(u16, Vec<RelocationArtifact>)>::new();

    for function in &artifact.functions {
        let section = u16::try_from(sections.len()).map_err(|_| fail("too many sections"))?;
        let size = as_u32(function.code.len(), "function")?;
        let mut body = function.code.clone();
        patch_addends(&mut body, &function.relocations)?;
        sections.push(Section {
            name: format!(".text.{}", function.name),
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_EXECINSTR,
            align: 4,
            data: body,
            link: 0,
            info: 0,
            entsize: 0,
            name_offset: 0,
            file_offset: 0,
        });
        if definitions
            .insert(function.name.clone(), (section, size, 0, STT_FUNC))
            .is_some()
        {
            return Err(fail(format!("duplicate symbol `{}`", function.name)));
        }
        pending_relocations.push((section, function.relocations.clone()));
    }

    for object in &artifact.data {
        let section = u16::try_from(sections.len()).map_err(|_| fail("too many sections"))?;
        let size = as_u32(object.bytes.len(), "data object")?;
        let mut bytes = object.bytes.clone();
        patch_addends(&mut bytes, &object.relocations)?;
        let is_bss = !object.read_only
            && object.relocations.is_empty()
            && object.bytes.iter().all(|byte| *byte == 0);
        sections.push(Section {
            name: format!(
                "{}.{}",
                if is_bss {
                    ".bss"
                } else if object.read_only {
                    ".rodata"
                } else {
                    ".data"
                },
                object.name
            ),
            kind: if is_bss { SHT_NOBITS } else { SHT_PROGBITS },
            flags: SHF_ALLOC | if object.read_only { 0 } else { SHF_WRITE },
            align: object.align,
            data: bytes,
            link: 0,
            info: 0,
            entsize: 0,
            name_offset: 0,
            file_offset: 0,
        });
        if definitions
            .insert(object.name.clone(), (section, size, 0, STT_OBJECT))
            .is_some()
        {
            return Err(fail(format!("duplicate symbol `{}`", object.name)));
        }
        pending_relocations.push((section, object.relocations.clone()));
    }

    // REL stores its addend in the target word. Undefined references are
    // ordinary global symbols, so standard ELF tooling can inspect them.
    let mut undefined = BTreeSet::new();
    for (_, relocations) in &pending_relocations {
        for relocation in relocations {
            if !definitions.contains_key(&relocation.target) {
                undefined.insert(relocation.target.clone());
            }
        }
    }
    let mut symbol_strings = vec![0];
    let mut symbols = vec![Symbol {
        name: String::new(),
        value: 0,
        size: 0,
        bind: STB_LOCAL,
        kind: STT_NOTYPE,
        section: 0,
    }];
    for (name, (section, size, value, kind)) in &definitions {
        if locals.contains(name) {
            symbols.push(Symbol {
                name: name.clone(),
                value: *value,
                size: *size,
                bind: STB_LOCAL,
                kind: *kind,
                section: *section,
            });
        }
    }
    let first_global = as_u32(symbols.len(), "symbol table")?;
    for (name, (section, size, value, kind)) in &definitions {
        if !locals.contains(name) {
            symbols.push(Symbol {
                name: name.clone(),
                value: *value,
                size: *size,
                bind: STB_GLOBAL,
                kind: *kind,
                section: *section,
            });
        }
    }
    for name in undefined {
        symbols.push(Symbol {
            name,
            value: 0,
            size: 0,
            bind: STB_GLOBAL,
            kind: STT_NOTYPE,
            section: 0,
        });
    }
    let mut symbol_indices = HashMap::new();
    let mut symbol_table = vec![0; 16];
    for (index, symbol) in symbols.iter().enumerate().skip(1) {
        let name_offset = string_offset(&mut symbol_strings, &symbol.name)?;
        write_symbol(&mut symbol_table, symbol, name_offset);
        if symbol_indices
            .insert(symbol.name.clone(), index as u32)
            .is_some()
        {
            return Err(fail(format!(
                "duplicate symbol-table name `{}`",
                symbol.name
            )));
        }
    }

    let symtab_index = u16::try_from(sections.len()).map_err(|_| fail("too many sections"))?;
    sections.push(Section {
        name: ".symtab".into(),
        kind: SHT_SYMTAB,
        flags: 0,
        align: 4,
        data: symbol_table,
        link: 0,
        info: first_global,
        entsize: 16,
        name_offset: 0,
        file_offset: 0,
    });
    let strtab_index = u16::try_from(sections.len()).map_err(|_| fail("too many sections"))?;
    sections.push(Section {
        name: ".strtab".into(),
        kind: SHT_STRTAB,
        flags: 0,
        align: 1,
        data: symbol_strings,
        link: 0,
        info: 0,
        entsize: 0,
        name_offset: 0,
        file_offset: 0,
    });
    sections[symtab_index as usize].link = u32::from(strtab_index);

    for (target, relocations) in pending_relocations {
        if relocations.is_empty() {
            continue;
        }
        let mut rel = Vec::new();
        for relocation in relocations {
            let index = *symbol_indices
                .get(&relocation.target)
                .ok_or_else(|| fail(format!("missing symbol `{}`", relocation.target)))?;
            append_u32(&mut rel, relocation.offset);
            append_u32(&mut rel, (index << 8) | R_SIA32_ABS32);
        }
        let target_name = &sections[target as usize].name;
        sections.push(Section {
            name: format!(".rel{target_name}"),
            kind: SHT_REL,
            flags: 0,
            align: 4,
            data: rel,
            link: u32::from(symtab_index),
            info: u32::from(target),
            entsize: 8,
            name_offset: 0,
            file_offset: 0,
        });
    }

    let mut section_names = vec![0];
    for section in sections.iter_mut().skip(1) {
        section.name_offset = string_offset(&mut section_names, &section.name)?;
    }
    let shstr_index = u16::try_from(sections.len()).map_err(|_| fail("too many sections"))?;
    let shstr_name = string_offset(&mut section_names, ".shstrtab")?;
    sections.push(Section {
        name: ".shstrtab".into(),
        kind: SHT_STRTAB,
        flags: 0,
        align: 1,
        data: section_names,
        link: 0,
        info: 0,
        entsize: 0,
        name_offset: shstr_name,
        file_offset: 0,
    });
    let mut out = vec![0; 52];
    out[0..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    out[16..18].copy_from_slice(&1u16.to_le_bytes()); // ET_REL
    out[18..20].copy_from_slice(&EM_SIA32.to_le_bytes());
    out[20..24].copy_from_slice(&1u32.to_le_bytes());
    out[40..42].copy_from_slice(&52u16.to_le_bytes());
    out[46..48].copy_from_slice(&40u16.to_le_bytes());
    out[48..50].copy_from_slice(
        &u16::try_from(sections.len())
            .map_err(|_| fail("too many sections"))?
            .to_le_bytes(),
    );
    out[50..52].copy_from_slice(&shstr_index.to_le_bytes());
    for section in sections.iter_mut().skip(1) {
        let aligned = align_to(out.len(), section.align as usize)?;
        out.resize(aligned, 0);
        section.file_offset = as_u32(out.len(), "section offset")?;
        if section.kind != SHT_NOBITS {
            out.extend_from_slice(&section.data);
        }
    }
    let shoff = as_u32(align_to(out.len(), 4)?, "section header offset")?;
    out.resize(shoff as usize, 0);
    for section in &sections {
        append_u32(&mut out, section.name_offset);
        append_u32(&mut out, section.kind);
        append_u32(&mut out, section.flags);
        append_u32(&mut out, 0); // address; ET_REL sections are not loaded
        append_u32(&mut out, section.file_offset);
        append_u32(&mut out, as_u32(section.data.len(), "section size")?);
        append_u32(&mut out, section.link);
        append_u32(&mut out, section.info);
        append_u32(&mut out, section.align);
        append_u32(&mut out, section.entsize);
    }
    out[32..36].copy_from_slice(&shoff.to_le_bytes());
    Ok(out)
}

fn patch_addends(bytes: &mut [u8], relocations: &[RelocationArtifact]) -> Result<(), Error> {
    let mut sites = HashSet::new();
    for relocation in relocations {
        let start = usize::try_from(relocation.offset).map_err(|_| fail("relocation offset"))?;
        let end = start
            .checked_add(4)
            .ok_or_else(|| fail("relocation range overflow"))?;
        if end > bytes.len() || !sites.insert(start) {
            return Err(fail("relocation is out of range or overlaps"));
        }
        let addend = i32::try_from(relocation.addend)
            .map_err(|_| fail("REL addend is outside signed 32-bit range"))?;
        bytes[start..end].copy_from_slice(&addend.to_le_bytes());
    }
    Ok(())
}

#[derive(Clone)]
struct ReadSection {
    name: String,
    kind: u32,
    flags: u32,
    offset: usize,
    size: usize,
    link: usize,
    info: usize,
    entsize: usize,
    align: usize,
}

#[derive(Clone)]
struct ReadSymbol {
    name: String,
    value: usize,
    size: usize,
    bind: u8,
    kind: u8,
    section: usize,
}

fn slice<'a>(bytes: &'a [u8], offset: usize, size: usize, what: &str) -> Result<&'a [u8], Error> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| fail(format!("{what} range overflow")))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| fail(format!("truncated {what}")))
}

fn u16_at(bytes: &[u8], offset: usize, what: &str) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(
        slice(bytes, offset, 2, what)?.try_into().unwrap(),
    ))
}

fn u32_at(bytes: &[u8], offset: usize, what: &str) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        slice(bytes, offset, 4, what)?.try_into().unwrap(),
    ))
}

fn c_string(strings: &[u8], offset: usize, what: &str) -> Result<String, Error> {
    let rest = strings
        .get(offset..)
        .ok_or_else(|| fail(format!("invalid {what} offset")))?;
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| fail(format!("unterminated {what}")))?;
    std::str::from_utf8(&rest[..end])
        .map(str::to_owned)
        .map_err(|_| fail(format!("non-UTF8 {what}")))
}

/// Read and validate the ELF32 subset emitted above.
pub(super) fn read(bytes: &[u8]) -> Result<Artifact, Error> {
    if bytes.len() > MAX_ELF_OBJECT_BYTES {
        return Err(fail("relocatable object exceeds 64 MiB parser budget"));
    }
    if slice(bytes, 0, 7, "ELF magic")? != b"\x7fELF\x01\x01\x01"
        || u16_at(bytes, 16, "ELF type")? != 1
        || u16_at(bytes, 18, "ELF machine")? != EM_SIA32
        || u32_at(bytes, 20, "ELF version")? != 1
        || u16_at(bytes, 40, "ELF header size")? != 52
        || u16_at(bytes, 46, "section header size")? != 40
    {
        return Err(fail("expected ELF32 little-endian SIA ET_REL object"));
    }
    if u32_at(bytes, 24, "entry")? != 0
        || u32_at(bytes, 28, "program header offset")? != 0
        || u16_at(bytes, 44, "program header size")? != 0
        || u16_at(bytes, 46, "section header size")? != 40
        || u16_at(bytes, 42, "program header count")? != 0
    {
        return Err(fail(
            "program headers or executable entry are unsupported in ET_REL",
        ));
    }
    let shoff = u32_at(bytes, 32, "section header offset")? as usize;
    let shnum = u16_at(bytes, 48, "section count")? as usize;
    let shstrndx = u16_at(bytes, 50, "section name table index")? as usize;
    if shnum < 2 || shstrndx >= shnum {
        return Err(fail("invalid section table"));
    }
    let table_size = shnum
        .checked_mul(40)
        .ok_or_else(|| fail("section table size overflow"))?;
    slice(bytes, shoff, table_size, "section header table")?;
    let mut raw_sections = Vec::with_capacity(shnum);
    for index in 0..shnum {
        let at = shoff + index * 40;
        raw_sections.push((
            u32_at(bytes, at, "section name offset")? as usize,
            u32_at(bytes, at + 4, "section type")?,
            u32_at(bytes, at + 8, "section flags")?,
            u32_at(bytes, at + 16, "section file offset")? as usize,
            u32_at(bytes, at + 20, "section size")? as usize,
            u32_at(bytes, at + 24, "section link")? as usize,
            u32_at(bytes, at + 28, "section info")? as usize,
            u32_at(bytes, at + 32, "section alignment")? as usize,
            u32_at(bytes, at + 36, "section entry size")? as usize,
        ));
    }
    if raw_sections[0].1 != SHT_NULL {
        return Err(fail("section zero is not SHT_NULL"));
    }
    let names_raw = raw_sections[shstrndx];
    if names_raw.1 != SHT_STRTAB {
        return Err(fail("section-name index is not a string table"));
    }
    let names = slice(bytes, names_raw.3, names_raw.4, "section-name table")?;
    let mut sections = Vec::with_capacity(shnum);
    let mut allocated_size = 0usize;
    for raw in raw_sections {
        if raw.1 != 8 {
            slice(bytes, raw.3, raw.4, "section contents")?;
        }
        if raw.7 > 1 && !raw.7.is_power_of_two() {
            return Err(fail("invalid section alignment"));
        }
        if raw.2 & SHF_ALLOC != 0 {
            allocated_size = allocated_size
                .checked_add(raw.4)
                .filter(|size| *size <= MAX_ELF_OBJECT_BYTES)
                .ok_or_else(|| fail("allocated sections exceed 64 MiB parser budget"))?;
        }
        sections.push(ReadSection {
            name: c_string(names, raw.0, "section name")?,
            kind: raw.1,
            flags: raw.2,
            offset: raw.3,
            size: raw.4,
            link: raw.5,
            info: raw.6,
            align: raw.7,
            entsize: raw.8,
        });
    }
    let symtab_index = sections
        .iter()
        .position(|section| section.kind == SHT_SYMTAB)
        .ok_or_else(|| fail("missing SHT_SYMTAB"))?;
    let symtab = &sections[symtab_index];
    if symtab.entsize != 16 || symtab.size % 16 != 0 || symtab.link >= sections.len() {
        return Err(fail("invalid symbol table"));
    }
    let strings_section = &sections[symtab.link];
    if strings_section.kind != SHT_STRTAB {
        return Err(fail("symbol-name link is not a string table"));
    }
    let strings = slice(
        bytes,
        strings_section.offset,
        strings_section.size,
        "symbol strings",
    )?;
    let mut symbols = Vec::with_capacity(symtab.size / 16);
    for index in 0..symtab.size / 16 {
        let at = symtab.offset + index * 16;
        let name = c_string(
            strings,
            u32_at(bytes, at, "symbol name")? as usize,
            "symbol name",
        )?;
        symbols.push(ReadSymbol {
            name,
            value: u32_at(bytes, at + 4, "symbol value")? as usize,
            size: u32_at(bytes, at + 8, "symbol size")? as usize,
            bind: slice(bytes, at + 12, 1, "symbol binding")?[0] >> 4,
            kind: slice(bytes, at + 12, 1, "symbol kind")?[0] & 0xf,
            section: u16_at(bytes, at + 14, "symbol section")? as usize,
        });
    }
    if symbols.is_empty() || !symbols[0].name.is_empty() || symbols[0].section != 0 {
        return Err(fail("invalid null symbol"));
    }
    let mut section_symbols = HashMap::<usize, usize>::new();
    let mut local_symbols = BTreeSet::new();
    let mut functions = Vec::new();
    let mut data = Vec::new();
    for (index, symbol) in symbols.iter().enumerate().skip(1) {
        if symbol.section == 0 {
            if symbol.bind != STB_GLOBAL || symbol.name.is_empty() {
                return Err(fail("unsupported undefined symbol"));
            }
            continue;
        }
        if symbol.section == 0xfff1 || symbol.section >= sections.len() {
            return Err(fail(format!("unsupported definition `{}`", symbol.name)));
        }
        let section = &sections[symbol.section];
        if section.flags & SHF_ALLOC == 0 || symbol.value != 0 || symbol.size != section.size {
            return Err(fail(format!(
                "unsupported symbol layout for `{}`",
                symbol.name
            )));
        }
        if section_symbols.insert(symbol.section, index).is_some() {
            return Err(fail(format!("multiple symbols share `{}`", section.name)));
        }
        if symbol.bind == STB_LOCAL {
            local_symbols.insert(symbol.name.clone());
        } else if symbol.bind != STB_GLOBAL {
            return Err(fail(format!(
                "unsupported symbol binding for `{}`",
                symbol.name
            )));
        }
        let contents = if section.kind == 8 {
            vec![0; section.size]
        } else if section.kind == SHT_PROGBITS {
            slice(bytes, section.offset, section.size, "allocated section")?.to_vec()
        } else {
            return Err(fail(format!(
                "unsupported allocated section `{}`",
                section.name
            )));
        };
        match symbol.kind {
            STT_FUNC if section.flags & SHF_EXECINSTR != 0 => functions.push(FunctionArtifact {
                name: symbol.name.clone(),
                code: contents,
                relocations: Vec::new(),
            }),
            STT_OBJECT if section.flags & SHF_EXECINSTR == 0 => data.push(DataArtifact {
                name: symbol.name.clone(),
                bytes: contents,
                align: u32::try_from(section.align)
                    .map_err(|_| fail("data alignment"))?
                    .max(1),
                read_only: section.flags & SHF_WRITE == 0,
                relocations: Vec::new(),
            }),
            _ => {
                return Err(fail(format!(
                    "unsupported symbol type for `{}`",
                    symbol.name
                )))
            }
        }
    }

    for section in sections.iter().filter(|section| section.kind == SHT_REL) {
        if section.entsize != 8
            || section.size % 8 != 0
            || section.link != symtab_index
            || section.info >= sections.len()
        {
            return Err(fail("invalid REL section"));
        }
        let symbol_index = *section_symbols
            .get(&section.info)
            .ok_or_else(|| fail("REL target section has no object symbol"))?;
        let owner = &symbols[symbol_index];
        for at in (section.offset..section.offset + section.size).step_by(8) {
            let offset = u32_at(bytes, at, "relocation offset")?;
            let info = u32_at(bytes, at + 4, "relocation info")?;
            if info & 0xff != R_SIA32_ABS32 {
                return Err(fail("unsupported relocation type"));
            }
            let target = symbols
                .get((info >> 8) as usize)
                .ok_or_else(|| fail("invalid relocation symbol index"))?;
            if target.name.is_empty() {
                return Err(fail("relocation references the null symbol"));
            }
            let start = offset as usize;
            let end = start
                .checked_add(4)
                .ok_or_else(|| fail("relocation range overflow"))?;
            if end > sections[section.info].size {
                return Err(fail("relocation site outside target section"));
            }
            let value = match sections[section.info].kind {
                SHT_PROGBITS => u32_at(bytes, sections[section.info].offset + start, "REL addend")?
                    as i32 as i64,
                8 => 0,
                _ => return Err(fail("unsupported REL target section")),
            };
            let relocation = RelocationArtifact {
                offset,
                target: target.name.clone(),
                addend: value,
            };
            match owner.kind {
                STT_FUNC => functions
                    .iter_mut()
                    .find(|function| function.name == owner.name)
                    .unwrap()
                    .relocations
                    .push(relocation),
                STT_OBJECT => data
                    .iter_mut()
                    .find(|object| object.name == owner.name)
                    .unwrap()
                    .relocations
                    .push(relocation),
                _ => return Err(fail("REL target symbol is not code or data")),
            }
        }
    }
    let artifact = Artifact {
        local_symbols: Some(local_symbols),
        target: crate::TARGET,
        functions,
        data,
    };
    artifact
        .validate()
        .map_err(|error| fail(error.to_string()))?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_readable_and_linkable_sia_elf_with_local_and_external_relocations() {
        let artifact = crate::compile_default(
            "extern int other(void); static int value=7; int main(void){ return other()+value; }",
        )
        .unwrap();
        let bytes = artifact.to_elf_bytes().unwrap();
        assert_eq!(&bytes[..7], b"\x7fELF\x01\x01\x01");
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), EM_SIA32);
        let decoded = Artifact::from_elf_bytes(&bytes).unwrap();
        let mut expected_functions = artifact.functions.clone();
        let mut expected_data = artifact.data.clone();
        for function in &mut expected_functions {
            patch_addends(&mut function.code, &function.relocations).unwrap();
        }
        for object in &mut expected_data {
            patch_addends(&mut object.bytes, &object.relocations).unwrap();
        }
        assert_eq!(decoded.functions, expected_functions);
        assert_eq!(decoded.data, expected_data);
        let linked = Artifact::link_images(&[decoded], 0x10000, "main", 0xc0000);
        assert!(
            linked.is_err(),
            "external unresolved symbol must remain link-time"
        );
    }

    #[test]
    fn represents_zero_initialized_writable_globals_as_nobits_bss() {
        let artifact =
            crate::compile_default("int counter; int main(void){return counter;}").unwrap();
        let bytes = artifact.to_elf_bytes().unwrap();
        let section_count = u16::from_le_bytes([bytes[48], bytes[49]]) as usize;
        let shoff = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
        let mut saw_nobits = false;
        for index in 1..section_count {
            let at = shoff + index * 40;
            if u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) == SHT_NOBITS {
                saw_nobits = true;
                assert_eq!(
                    u32::from_le_bytes(bytes[at + 8..at + 12].try_into().unwrap()),
                    SHF_ALLOC | SHF_WRITE
                );
            }
        }
        assert!(
            saw_nobits,
            "zero-initialized writable storage should use SHT_NOBITS"
        );
        assert_eq!(
            Artifact::from_elf_bytes(&bytes).unwrap().data[0].bytes,
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn linked_sia_elf_executable_loads_code_data_and_zero_fill_segments() {
        let main = crate::compile_default(
            "extern int helper(void); extern int counter; int main(void){return helper()+counter==42?0:1;}",
        )
        .unwrap();
        let helper = crate::compile_default("int counter; int helper(void){return 42;}").unwrap();
        let expected = Artifact::link_images(&[main, helper], 0x10000, "main", 0xc0000).unwrap();
        let bytes = expected.to_elf_executable_bytes().unwrap();
        assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), ET_EXEC);
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), EM_SIA32);
        let loaded = LinkedImage::from_elf_executable_bytes(&bytes, 0xc0000).unwrap();
        assert_eq!(loaded.base, expected.base);
        assert_eq!(loaded.entry, expected.entry);
        assert_eq!(loaded.bytes, expected.bytes);
        assert_eq!(loaded.regions.len(), expected.regions.len());
        assert!(loaded.regions.iter().any(|region| region.zero_fill));
    }

    #[test]
    fn rejects_elf_executable_with_invalid_load_permissions_or_entry() {
        let artifact = crate::compile_default("int main(void){return 0;}").unwrap();
        let image = artifact.link_image(0x10000, "main", 0xc0000).unwrap();
        let bytes = image.to_elf_executable_bytes().unwrap();
        let mut invalid_entry = bytes.clone();
        invalid_entry[24..28].copy_from_slice(&0u32.to_le_bytes());
        assert!(LinkedImage::from_elf_executable_bytes(&invalid_entry, 0xc0000).is_err());
        let mut invalid_segment = bytes;
        invalid_segment[52 + 24..52 + 28].copy_from_slice(&8u32.to_le_bytes());
        assert!(LinkedImage::from_elf_executable_bytes(&invalid_segment, 0xc0000).is_err());
    }

    #[test]
    fn rejects_foreign_and_truncated_elf() {
        let artifact = crate::compile_default("int main(void){return 0;}").unwrap();
        let bytes = artifact.to_elf_bytes().unwrap();
        assert!(Artifact::from_elf_bytes(&bytes[..40]).is_err());
        let mut foreign = bytes;
        foreign[18..20].copy_from_slice(&3u16.to_le_bytes());
        assert!(Artifact::from_elf_bytes(&foreign).is_err());
    }
}
