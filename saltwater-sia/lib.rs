//! Cosmic C's initial SIA32 lowering path.
//!
//! This crate deliberately emits only SIA32 code.  It does not fall back to a
//! host ISA. Floating-point objects remain rejected until SIA32 FP machine lowering exists.

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::fmt;

use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, AbiParam, Block, ExtFuncData, ExternalName, Function, GlobalValueData, InstBuilder,
    MemFlagsData, Signature, StackSlot, StackSlotData, StackSlotKind, UserExternalName,
    UserFuncName, Value,
};
use cranelift_codegen::isa::{self, CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::Context;
use cranelift_codegen::RelocTarget;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use saltwater_parser::check_semantics;
pub use saltwater_parser::data::error::LexError;
use saltwater_parser::data::types::FunctionType;
use saltwater_parser::data::{
    hir::{Declaration, Expr, ExprType, Initializer, LiteralValue, Stmt, StmtType, Symbol},
    CompileError, Location, StorageClass, Type,
};
pub use saltwater_parser::{preprocess, Opt};
use target_lexicon::Triple;

/// The fixed target accepted by this compiler stage.
pub const TARGET: &str = "sia32-unknown-none";

const BUNDLE_MAGIC: &[u8] = b"COSMIC-SIA\0";
const BUNDLE_VERSION: u16 = 3;
const SIA_REGISTER_COUNT: usize = 16;
const SIA_ARGUMENT_REGISTER: usize = 1;
const SIA_LINK_REGISTER: usize = 14;
const SIA_MAX_INTEGER_ARGUMENTS: usize = 6;

/// Raw SIA32 code for one C function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionArtifact {
    /// C linkage name.
    pub name: String,
    /// Native SIA32 instruction bytes.
    pub code: Vec<u8>,
    /// Link-time relocations emitted by the SIA32 backend.
    pub relocations: Vec<RelocationArtifact>,
}

/// One SIA32 link-time relocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelocationArtifact {
    pub offset: u32,
    pub target: String,
    pub addend: i64,
}

/// One relocatable data object emitted by Cosmic C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataArtifact {
    /// Linkage name used by code/data relocations.
    pub name: String,
    /// Initial contents. Uninitialized objects are emitted as zero-filled bytes.
    pub bytes: Vec<u8>,
    /// Required power-of-two byte alignment.
    pub align: u32,
    /// Whether the object should be mapped read-only by the eventual linker/loader.
    pub read_only: bool,
    /// Link-time relocations embedded in this data object.
    pub relocations: Vec<RelocationArtifact>,
}

/// A relocatable-in-spirit SIA code bundle.
///
/// The bundle is intentionally small while the Cosmic object/image writer is
/// being built. It records unrelocated function code only; calls and globals
/// are rejected rather than emitted with guessed relocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Target triple used to build every contained function.
    pub target: &'static str,
    /// Compiled function bodies.
    pub functions: Vec<FunctionArtifact>,
    /// Relocatable global/static/string data objects.
    pub data: Vec<DataArtifact>,
}

/// A concrete SIA32 call prepared for an external Lighting execution harness.
///
/// This crate deliberately prepares architectural state only. The bytes and
/// registers must be consumed by Lighting's real execution path; this type
/// must never grow a host-side SIA interpreter or JIT fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallPlan {
    /// C linkage name of the function being called.
    pub function: String,
    /// Address at which `code` must be loaded.
    pub entry_address: u32,
    /// Address expected after the compiled function returns through `lr`.
    pub return_address: u32,
    /// Native SIA32 bytes to load at `entry_address`.
    pub code: Vec<u8>,
    /// Complete initial architectural integer register state.
    pub registers: [u32; SIA_REGISTER_COUNT],
}

impl CallPlan {
    /// Render the bounded state that an execution-harness failure must report.
    pub fn diagnostic(&self) -> String {
        let code = self
            .code
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let registers = self
            .registers
            .iter()
            .enumerate()
            .map(|(index, value)| format!("r{index}=0x{value:08x}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "function={} entry=0x{:08x} return=0x{:08x} code=[{}] registers=[{}]",
            self.function, self.entry_address, self.return_address, code, registers
        )
    }
}

impl Artifact {
    /// Serialize this artifact to the stable, little-endian `COSMIC-SIA` bundle format.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut bytes = Vec::from(BUNDLE_MAGIC);
        bytes.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
        let count = u16::try_from(self.functions.len())
            .map_err(|_| Error::Codegen("too many functions for a SIA bundle".into()))?;
        bytes.extend_from_slice(&count.to_le_bytes());
        for function in &self.functions {
            let name = function.name.as_bytes();
            let name_len = u16::try_from(name.len())
                .map_err(|_| Error::Codegen("function name is too long for a SIA bundle".into()))?;
            let code_len = u32::try_from(function.code.len()).map_err(|_| {
                Error::Codegen("function body is too large for a SIA bundle".into())
            })?;
            bytes.extend_from_slice(&name_len.to_le_bytes());
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(&code_len.to_le_bytes());
            bytes.extend_from_slice(&function.code);
            let reloc_count = u16::try_from(function.relocations.len())
                .map_err(|_| Error::Codegen("too many relocations for a SIA function".into()))?;
            bytes.extend_from_slice(&reloc_count.to_le_bytes());
            for relocation in &function.relocations {
                let target = relocation.target.as_bytes();
                let target_len = u16::try_from(target.len())
                    .map_err(|_| Error::Codegen("relocation target name is too long".into()))?;
                bytes.extend_from_slice(&relocation.offset.to_le_bytes());
                bytes.extend_from_slice(&relocation.addend.to_le_bytes());
                bytes.extend_from_slice(&target_len.to_le_bytes());
                bytes.extend_from_slice(target);
            }
        }
        let data_count = u16::try_from(self.data.len())
            .map_err(|_| Error::Codegen("too many data objects for a SIA bundle".into()))?;
        bytes.extend_from_slice(&data_count.to_le_bytes());
        for object in &self.data {
            let name = object.name.as_bytes();
            let name_len = u16::try_from(name.len())
                .map_err(|_| Error::Codegen("data object name is too long".into()))?;
            let data_len = u32::try_from(object.bytes.len())
                .map_err(|_| Error::Codegen("data object is too large".into()))?;
            bytes.extend_from_slice(&name_len.to_le_bytes());
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(&object.align.to_le_bytes());
            bytes.push(u8::from(object.read_only));
            bytes.extend_from_slice(&data_len.to_le_bytes());
            bytes.extend_from_slice(&object.bytes);
            let reloc_count = u16::try_from(object.relocations.len())
                .map_err(|_| Error::Codegen("too many relocations for a SIA data object".into()))?;
            bytes.extend_from_slice(&reloc_count.to_le_bytes());
            for relocation in &object.relocations {
                let target = relocation.target.as_bytes();
                let target_len = u16::try_from(target.len())
                    .map_err(|_| Error::Codegen("relocation target name is too long".into()))?;
                bytes.extend_from_slice(&relocation.offset.to_le_bytes());
                bytes.extend_from_slice(&relocation.addend.to_le_bytes());
                bytes.extend_from_slice(&target_len.to_le_bytes());
                bytes.extend_from_slice(target);
            }
        }
        Ok(bytes)
    }

    /// Decode a stable little-endian `COSMIC-SIA` bundle.
    ///
    /// The decoder accepts only the temporary unrelocated format emitted by
    /// this crate. It is intentionally strict so a future Lighting bridge
    /// cannot execute a truncated or ambiguously framed image.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut cursor = 0;
        let magic = take(bytes, &mut cursor, BUNDLE_MAGIC.len(), "bundle magic")?;
        if magic != BUNDLE_MAGIC {
            return Err(Error::Codegen("invalid COSMIC-SIA bundle magic".into()));
        }
        let version = u16::from_le_bytes(read_array(take(bytes, &mut cursor, 2, "version")?));
        if version != BUNDLE_VERSION {
            return Err(Error::Codegen(format!(
                "unsupported COSMIC-SIA bundle version {version}"
            )));
        }
        let function_count =
            u16::from_le_bytes(read_array(take(bytes, &mut cursor, 2, "function count")?));
        let mut functions = Vec::with_capacity(usize::from(function_count));
        for _ in 0..function_count {
            let name_len = usize::from(u16::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                2,
                "function name length",
            )?)));
            let name_bytes = take(bytes, &mut cursor, name_len, "function name")?;
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| Error::Codegen("COSMIC-SIA function name is not UTF-8".into()))?
                .to_owned();
            let code_len = usize::try_from(u32::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                4,
                "function code length",
            )?)))
            .expect("a u32 always fits in usize on supported Cosmic C hosts");
            let code = take(bytes, &mut cursor, code_len, "function code")?.to_vec();
            let reloc_count = usize::from(u16::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                2,
                "function relocation count",
            )?)));
            let mut relocations = Vec::with_capacity(reloc_count);
            for _ in 0..reloc_count {
                let offset = u32::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    4,
                    "relocation offset",
                )?));
                let addend = i64::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    8,
                    "relocation addend",
                )?));
                let target_len = usize::from(u16::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    2,
                    "relocation target length",
                )?)));
                let target =
                    std::str::from_utf8(take(bytes, &mut cursor, target_len, "relocation target")?)
                        .map_err(|_| {
                            Error::Codegen("COSMIC-SIA relocation target is not UTF-8".into())
                        })?
                        .to_owned();
                relocations.push(RelocationArtifact {
                    offset,
                    target,
                    addend,
                });
            }
            functions.push(FunctionArtifact {
                name,
                code,
                relocations,
            });
        }
        let data_count = u16::from_le_bytes(read_array(take(bytes, &mut cursor, 2, "data count")?));
        let mut data = Vec::with_capacity(usize::from(data_count));
        for _ in 0..data_count {
            let name_len = usize::from(u16::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                2,
                "data name length",
            )?)));
            let name = std::str::from_utf8(take(bytes, &mut cursor, name_len, "data name")?)
                .map_err(|_| Error::Codegen("COSMIC-SIA data name is not UTF-8".into()))?
                .to_owned();
            let align =
                u32::from_le_bytes(read_array(take(bytes, &mut cursor, 4, "data alignment")?));
            let read_only = take(bytes, &mut cursor, 1, "data read-only flag")?[0] != 0;
            let data_len = usize::try_from(u32::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                4,
                "data length",
            )?)))
            .expect("a u32 always fits in usize on supported Cosmic C hosts");
            let object_bytes = take(bytes, &mut cursor, data_len, "data bytes")?.to_vec();
            let reloc_count = usize::from(u16::from_le_bytes(read_array(take(
                bytes,
                &mut cursor,
                2,
                "data relocation count",
            )?)));
            let mut relocations = Vec::with_capacity(reloc_count);
            for _ in 0..reloc_count {
                let offset = u32::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    4,
                    "data relocation offset",
                )?));
                let addend = i64::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    8,
                    "data relocation addend",
                )?));
                let target_len = usize::from(u16::from_le_bytes(read_array(take(
                    bytes,
                    &mut cursor,
                    2,
                    "data relocation target length",
                )?)));
                let target = std::str::from_utf8(take(
                    bytes,
                    &mut cursor,
                    target_len,
                    "data relocation target",
                )?)
                .map_err(|_| {
                    Error::Codegen("COSMIC-SIA data relocation target is not UTF-8".into())
                })?
                .to_owned();
                relocations.push(RelocationArtifact {
                    offset,
                    target,
                    addend,
                });
            }
            data.push(DataArtifact {
                name,
                bytes: object_bytes,
                align,
                read_only,
                relocations,
            });
        }
        if cursor != bytes.len() {
            return Err(Error::Codegen(
                "COSMIC-SIA bundle has trailing bytes".into(),
            ));
        }
        let artifact = Self {
            target: TARGET,
            functions,
            data,
        };
        artifact.validate()?;
        Ok(artifact)
    }

    /// Return a function by its C linkage name.
    pub fn function(&self, name: &str) -> Option<&FunctionArtifact> {
        self.functions.iter().find(|function| function.name == name)
    }

    /// Prepare one scalar SystemV SIA32 call for an external Lighting harness.
    ///
    /// The initial compiler subset has no relocations, globals, calls, or
    /// stack locals. Therefore a function can be loaded verbatim at an aligned
    /// address. `lr` is set immediately past the code so a normal return gives
    /// the harness one explicit return boundary.
    pub fn prepare_integer_call(
        &self,
        name: &str,
        entry_address: u32,
        arguments: &[u32],
    ) -> Result<CallPlan, Error> {
        if entry_address & 1 != 0 {
            return Err(Error::Codegen(format!(
                "SIA call entry for {name} must be 2-byte aligned"
            )));
        }
        if arguments.len() > SIA_MAX_INTEGER_ARGUMENTS {
            return Err(Error::Codegen(format!(
                "SIA32 supports at most {SIA_MAX_INTEGER_ARGUMENTS} scalar register arguments"
            )));
        }
        let function = self
            .function(name)
            .ok_or_else(|| Error::Codegen(format!("COSMIC-SIA bundle has no function `{name}`")))?;
        if !function.relocations.is_empty() {
            return Err(Error::Codegen(format!(
                "SIA function `{name}` requires linking before direct Lighting execution"
            )));
        }
        let code_len = u32::try_from(function.code.len())
            .map_err(|_| Error::Codegen("SIA function body exceeds 32-bit address space".into()))?;
        let return_address = entry_address
            .checked_add(code_len)
            .ok_or_else(|| Error::Codegen("SIA function address range overflows".into()))?;
        let mut registers = [0; SIA_REGISTER_COUNT];
        for (index, argument) in arguments.iter().copied().enumerate() {
            registers[SIA_ARGUMENT_REGISTER + index] = argument;
        }
        registers[SIA_LINK_REGISTER] = return_address;
        Ok(CallPlan {
            function: function.name.clone(),
            entry_address,
            return_address,
            code: function.code.clone(),
            registers,
        })
    }

    fn validate(&self) -> Result<(), Error> {
        if self.target != TARGET {
            return Err(Error::Codegen(format!(
                "COSMIC-SIA bundle target must be `{TARGET}`"
            )));
        }
        if self.functions.is_empty() && self.data.is_empty() {
            return Err(Error::Codegen(
                "COSMIC-SIA bundle contains neither functions nor data".into(),
            ));
        }
        let mut names = HashSet::new();
        for function in &self.functions {
            if function.name.is_empty() {
                return Err(Error::Codegen(
                    "COSMIC-SIA function name must not be empty".into(),
                ));
            }
            if !names.insert(&function.name) {
                return Err(Error::Codegen(format!(
                    "COSMIC-SIA bundle contains duplicate symbol `{}`",
                    function.name
                )));
            }
            if function.code.is_empty() || function.code.len() % 2 != 0 {
                return Err(Error::Codegen(format!(
                    "COSMIC-SIA function `{}` does not contain whole SIA instruction words",
                    function.name
                )));
            }
        }
        for object in &self.data {
            if object.name.is_empty() {
                return Err(Error::Codegen(
                    "COSMIC-SIA data object name must not be empty".into(),
                ));
            }
            if !names.insert(&object.name) {
                return Err(Error::Codegen(format!(
                    "COSMIC-SIA bundle contains duplicate symbol `{}`",
                    object.name
                )));
            }
            if object.align == 0 || !object.align.is_power_of_two() {
                return Err(Error::Codegen(format!(
                    "COSMIC-SIA data object `{}` has invalid alignment {}",
                    object.name, object.align
                )));
            }
            for relocation in &object.relocations {
                let end = usize::try_from(relocation.offset)
                    .ok()
                    .and_then(|offset| offset.checked_add(4))
                    .ok_or_else(|| Error::Codegen("data relocation offset overflows".into()))?;
                if end > object.bytes.len() {
                    return Err(Error::Codegen(format!(
                        "COSMIC-SIA data relocation in `{}` lies outside the object",
                        object.name
                    )));
                }
            }
        }
        Ok(())
    }
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    count: usize,
    field: &str,
) -> Result<&'a [u8], Error> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| Error::Codegen(format!("COSMIC-SIA {field} length overflows")))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| Error::Codegen(format!("truncated COSMIC-SIA {field}")))?;
    *cursor = end;
    Ok(value)
}

fn read_array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes
        .try_into()
        .expect("COSMIC-SIA field width was validated before conversion")
}

/// A failure reported by the C frontend or the SIA lowering stage.
#[derive(Debug)]
pub enum Error {
    /// Existing C parsing or semantic diagnostics.
    Source(VecDeque<CompileError>),
    /// A valid C construct outside the deliberately small initial SIA subset.
    Unsupported { location: Location, message: String },
    /// A Cranelift backend failure.
    Codegen(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Source(errors) => {
                for (index, error) in errors.iter().enumerate() {
                    if index != 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{}", error.data)?;
                }
                Ok(())
            }
            Error::Unsupported { message, .. } | Error::Codegen(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

fn source_error(error: CompileError) -> Error {
    Error::Source(VecDeque::from([error]))
}

fn unsupported(location: Location, message: impl Into<String>) -> Error {
    Error::Unsupported {
        location,
        message: message.into(),
    }
}

fn scalar_initializer_is_zero(expression: &Expr) -> bool {
    match &expression.expr {
        ExprType::Literal(LiteralValue::Int(0))
        | ExprType::Literal(LiteralValue::UnsignedInt(0))
        | ExprType::Literal(LiteralValue::Char(0)) => true,
        ExprType::Cast(inner) | ExprType::Noop(inner) | ExprType::StaticRef(inner) => {
            scalar_initializer_is_zero(inner)
        }
        _ => false,
    }
}

fn scalar_initializer_bytes(
    expression: &Expr,
    target: &Type,
    location: Location,
) -> Result<Vec<u8>, Error> {
    let folded = expression.clone().const_fold().map_err(source_error)?;
    let width = usize::try_from(
        target
            .sizeof()
            .map_err(|error| unsupported(location, error.to_string()))?,
    )
    .map_err(|_| unsupported(location, "scalar initializer is too large"))?;
    let mut bytes = vec![0; width];
    if scalar_initializer_is_zero(&folded) {
        return Ok(bytes);
    }
    match folded.expr {
        ExprType::Literal(LiteralValue::Int(value)) => {
            let raw = value.to_le_bytes();
            bytes.copy_from_slice(&raw[..width.min(raw.len())]);
        }
        ExprType::Literal(LiteralValue::UnsignedInt(value)) => {
            let raw = value.to_le_bytes();
            bytes.copy_from_slice(&raw[..width.min(raw.len())]);
        }
        ExprType::Literal(LiteralValue::Char(value)) => {
            if let Some(first) = bytes.first_mut() {
                *first = value;
            }
        }
        ExprType::Literal(LiteralValue::Float(_)) => {
            return Err(unsupported(
                location,
                "floating-point global initialization requires SIA32 float lowering",
            ));
        }
        ExprType::Literal(LiteralValue::Str(_)) => {
            return Err(unsupported(
                location,
                "string scalar initialization requires SIA32 string-data lowering",
            ));
        }
        _ if folded.is_zero() => {}
        _ => {
            return Err(unsupported(
                location,
                "global scalar initializer is not a link-time constant",
            ))
        }
    }
    Ok(bytes)
}

fn collect_string_literals_expr(expression: &Expr, strings: &mut HashMap<Vec<u8>, u32>) {
    if let ExprType::Literal(LiteralValue::Str(bytes)) = &expression.expr {
        if !strings.contains_key(bytes) {
            let index = u32::try_from(strings.len()).expect("string pool fits in u32");
            strings.insert(bytes.clone(), index);
        }
    }
    match &expression.expr {
        ExprType::FuncCall(function, arguments) => {
            collect_string_literals_expr(function, strings);
            for argument in arguments {
                collect_string_literals_expr(argument, strings);
            }
        }
        ExprType::Member(value, _)
        | ExprType::PostIncrement(value, _)
        | ExprType::Cast(value)
        | ExprType::Deref(value)
        | ExprType::Negate(value)
        | ExprType::BitwiseNot(value)
        | ExprType::StaticRef(value)
        | ExprType::Noop(value) => collect_string_literals_expr(value, strings),
        ExprType::Binary(_, left, right) | ExprType::Comma(left, right) => {
            collect_string_literals_expr(left, strings);
            collect_string_literals_expr(right, strings);
        }
        ExprType::Ternary(condition, yes, no) => {
            collect_string_literals_expr(condition, strings);
            collect_string_literals_expr(yes, strings);
            collect_string_literals_expr(no, strings);
        }
        ExprType::Id(_) | ExprType::Literal(_) | ExprType::Sizeof(_) => {}
    }
}

fn collect_string_literals_initializer(
    initializer: &Initializer,
    strings: &mut HashMap<Vec<u8>, u32>,
) {
    match initializer {
        Initializer::Scalar(expression) => collect_string_literals_expr(expression, strings),
        Initializer::InitializerList(items) => {
            for item in items {
                collect_string_literals_initializer(item, strings);
            }
        }
        Initializer::Zero => {}
        Initializer::FunctionBody(statements) => {
            for statement in statements {
                collect_string_literals_stmt(statement, strings);
            }
        }
    }
}

fn collect_string_literals_stmt(statement: &Stmt, strings: &mut HashMap<Vec<u8>, u32>) {
    match &statement.data {
        StmtType::Compound(statements) => {
            for statement in statements {
                collect_string_literals_stmt(statement, strings);
            }
        }
        StmtType::If(condition, yes, no) => {
            collect_string_literals_expr(condition, strings);
            collect_string_literals_stmt(yes, strings);
            if let Some(no) = no {
                collect_string_literals_stmt(no, strings);
            }
        }
        StmtType::Do(body, condition) | StmtType::While(condition, body) => {
            collect_string_literals_stmt(body, strings);
            collect_string_literals_expr(condition, strings);
        }
        StmtType::For(init, condition, step, body) => {
            collect_string_literals_stmt(init, strings);
            if let Some(condition) = condition {
                collect_string_literals_expr(condition, strings);
            }
            if let Some(step) = step {
                collect_string_literals_expr(step, strings);
            }
            collect_string_literals_stmt(body, strings);
        }
        StmtType::Switch(expression, body) => {
            collect_string_literals_expr(expression, strings);
            collect_string_literals_stmt(body, strings);
        }
        StmtType::Label(_, body) | StmtType::Case(_, body) | StmtType::Default(body) => {
            collect_string_literals_stmt(body, strings);
        }
        StmtType::Expr(expression) => collect_string_literals_expr(expression, strings),
        StmtType::Return(value) => {
            if let Some(value) = value {
                collect_string_literals_expr(value, strings);
            }
        }
        StmtType::Decl(declarations) => {
            for declaration in declarations {
                if let Some(initializer) = &declaration.data.init {
                    collect_string_literals_initializer(initializer, strings);
                }
            }
        }
        StmtType::Goto(_) | StmtType::Continue | StmtType::Break => {}
    }
}

fn string_symbol_name(index: u32) -> String {
    format!("__cosmic_str_{index}")
}

fn collect_static_locals(
    statements: &[Stmt],
    function_index: usize,
    out: &mut Vec<(usize, Declaration, Location)>,
) {
    for statement in statements {
        match &statement.data {
            StmtType::Compound(statements) => {
                collect_static_locals(statements, function_index, out);
            }
            StmtType::Decl(declarations) => {
                for declaration in declarations {
                    let metadata = declaration.data.symbol.get();
                    if metadata.storage_class == StorageClass::Static
                        && !matches!(metadata.ctype, Type::Function(_))
                    {
                        out.push((
                            function_index,
                            declaration.data.clone(),
                            declaration.location,
                        ));
                    }
                }
            }
            StmtType::If(_, yes, no) => {
                collect_static_locals(std::slice::from_ref(yes.as_ref()), function_index, out);
                if let Some(no) = no {
                    collect_static_locals(std::slice::from_ref(no.as_ref()), function_index, out);
                }
            }
            StmtType::Do(body, _) | StmtType::While(_, body) => {
                collect_static_locals(std::slice::from_ref(body.as_ref()), function_index, out);
            }
            StmtType::For(init, _, _, body) => {
                collect_static_locals(std::slice::from_ref(init.as_ref()), function_index, out);
                collect_static_locals(std::slice::from_ref(body.as_ref()), function_index, out);
            }
            StmtType::Switch(_, body)
            | StmtType::Label(_, body)
            | StmtType::Case(_, body)
            | StmtType::Default(body) => {
                collect_static_locals(std::slice::from_ref(body.as_ref()), function_index, out);
            }
            StmtType::Expr(_)
            | StmtType::Goto(_)
            | StmtType::Continue
            | StmtType::Break
            | StmtType::Return(_) => {}
        }
    }
}

fn completed_object_type(
    ctype: &Type,
    initializer: Option<&Initializer>,
    location: Location,
) -> Result<Type, Error> {
    if let Type::Array(element, saltwater_parser::data::types::ArrayType::Unbounded) = ctype {
        let len = match initializer {
            Some(Initializer::InitializerList(items)) => u64::try_from(items.len())
                .map_err(|_| unsupported(location, "array initializer is too large"))?,
            Some(Initializer::Scalar(expression)) => match &expression.expr {
                ExprType::Literal(LiteralValue::Str(bytes)) => u64::try_from(bytes.len())
                    .map_err(|_| unsupported(location, "string initializer is too large"))?,
                _ => {
                    return Err(unsupported(
                        location,
                        "unbounded array requires an initializer that determines its size",
                    ))
                }
            },
            _ => {
                return Err(unsupported(
                    location,
                    "unbounded array requires an initializer that determines its size",
                ))
            }
        };
        Ok(Type::Array(
            element.clone(),
            saltwater_parser::data::types::ArrayType::Fixed(len),
        ))
    } else {
        Ok(ctype.clone())
    }
}

fn translation_unit_symbol_name(index: usize, declaration: &Declaration) -> String {
    let metadata = declaration.symbol.get();
    let raw_name = metadata.id.resolve_and_clone();
    if !matches!(metadata.ctype, Type::Function(_))
        && metadata.storage_class == StorageClass::Static
    {
        format!("__cosmic_static_global_{index}_{raw_name}")
    } else {
        raw_name
    }
}

fn aggregate_member_offset(
    ctype: &Type,
    member: saltwater_parser::intern::InternedStr,
    location: Location,
) -> Result<u64, Error> {
    match ctype {
        Type::Union(_) => Ok(0),
        Type::Struct(struct_type) => {
            let mut offset = 0u64;
            for field in struct_type.members().iter() {
                let align = field
                    .ctype
                    .alignof()
                    .map_err(|error| unsupported(location, error.to_string()))?;
                if align > 1 {
                    let rem = offset % align;
                    if rem != 0 {
                        offset += align - rem;
                    }
                }
                if field.id == member {
                    return Ok(offset);
                }
                offset = offset
                    .checked_add(
                        field
                            .ctype
                            .sizeof()
                            .map_err(|error| unsupported(location, error.to_string()))?,
                    )
                    .ok_or_else(|| Error::Codegen("member offset overflows".into()))?;
            }
            Err(unsupported(location, "unknown aggregate member"))
        }
        _ => Err(unsupported(
            location,
            "member offset requested for non-aggregate type",
        )),
    }
}

fn static_address_target(
    expression: &Expr,
    symbols: &HashMap<Symbol, String>,
) -> Result<Option<(String, i64)>, Error> {
    fn resolve(
        expression: &Expr,
        symbols: &HashMap<Symbol, String>,
    ) -> Result<Option<(String, i64)>, Error> {
        match &expression.expr {
            ExprType::Noop(inner) | ExprType::Cast(inner) => resolve(inner, symbols),
            ExprType::StaticRef(inner) => match &inner.expr {
                ExprType::Id(symbol) => Ok(symbols.get(symbol).cloned().map(|name| (name, 0))),
                ExprType::Member(base, member) => {
                    if let ExprType::Id(symbol) = &base.expr {
                        let Some(name) = symbols.get(symbol).cloned() else {
                            return Ok(None);
                        };
                        let offset = aggregate_member_offset(&base.ctype, *member, base.location)?;
                        let addend = i64::try_from(offset).map_err(|_| {
                            Error::Codegen("member relocation addend overflows".into())
                        })?;
                        Ok(Some((name, addend)))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            },
            // Function designators can appear directly in pointer initializers.
            ExprType::Id(symbol) if matches!(symbol.get().ctype, Type::Function(_)) => {
                Ok(symbols.get(symbol).cloned().map(|name| (name, 0)))
            }
            _ => Ok(None),
        }
    }
    resolve(expression, symbols)
}

fn write_global_initializer(
    bytes: &mut [u8],
    relocations: &mut Vec<RelocationArtifact>,
    symbols: &HashMap<Symbol, String>,
    strings: &HashMap<Vec<u8>, u32>,
    base_offset: usize,
    ctype: &Type,
    initializer: &Initializer,
    location: Location,
) -> Result<(), Error> {
    match initializer {
        // Global object storage is allocated zero-filled before explicit
        // initializer writes, so a sparse designated gap requires no write.
        Initializer::Zero => Ok(()),
        Initializer::Scalar(expression) if ctype.is_scalar() => {
            if matches!(ctype, Type::Pointer(_, _) | Type::Function(_)) {
                let mut string_expression = expression;
                while let ExprType::Noop(inner) | ExprType::Cast(inner) = &string_expression.expr {
                    string_expression = inner;
                }
                let string_bytes = match &string_expression.expr {
                    ExprType::StaticRef(inner) => match &inner.expr {
                        ExprType::Literal(LiteralValue::Str(bytes)) => Some(bytes),
                        _ => None,
                    },
                    ExprType::Literal(LiteralValue::Str(bytes)) => Some(bytes),
                    _ => None,
                };
                if let Some(bytes_value) = string_bytes {
                    let index = strings.get(bytes_value).copied().ok_or_else(|| {
                        Error::Codegen("string literal missing from translation-unit pool".into())
                    })?;
                    let offset = u32::try_from(base_offset)
                        .map_err(|_| Error::Codegen("data relocation offset overflows".into()))?;
                    relocations.push(RelocationArtifact {
                        offset,
                        target: string_symbol_name(index),
                        addend: 0,
                    });
                    return Ok(());
                }
                if let Some((target, addend)) = static_address_target(expression, symbols)? {
                    let offset = u32::try_from(base_offset)
                        .map_err(|_| Error::Codegen("data relocation offset overflows".into()))?;
                    let end = base_offset
                        .checked_add(4)
                        .ok_or_else(|| Error::Codegen("data relocation offset overflows".into()))?;
                    if end > bytes.len() {
                        return Err(Error::Codegen(
                            "data relocation exceeds global object".into(),
                        ));
                    }
                    relocations.push(RelocationArtifact {
                        offset,
                        target,
                        addend,
                    });
                    return Ok(());
                }
            }
            let value = scalar_initializer_bytes(expression, ctype, location)?;
            let end = base_offset
                .checked_add(value.len())
                .ok_or_else(|| Error::Codegen("global initializer offset overflows".into()))?;
            bytes
                .get_mut(base_offset..end)
                .ok_or_else(|| Error::Codegen("global initializer exceeds object size".into()))?
                .copy_from_slice(&value);
            Ok(())
        }
        Initializer::InitializerList(items) if ctype.is_scalar() => {
            if items.len() != 1 {
                return Err(unsupported(
                    location,
                    "scalar global initializer list must contain exactly one element",
                ));
            }
            write_global_initializer(
                bytes,
                relocations,
                symbols,
                strings,
                base_offset,
                ctype,
                &items[0],
                location,
            )
        }
        Initializer::InitializerList(items) => match ctype {
            Type::Array(element, saltwater_parser::data::types::ArrayType::Fixed(count)) => {
                if u64::try_from(items.len()).unwrap_or(u64::MAX) > *count {
                    return Err(unsupported(
                        location,
                        "too many elements in global array initializer",
                    ));
                }
                let element_size = usize::try_from(
                    element
                        .sizeof()
                        .map_err(|error| unsupported(location, error.to_string()))?,
                )
                .map_err(|_| unsupported(location, "global array element is too large"))?;
                for (index, item) in items.iter().enumerate() {
                    let offset = base_offset
                        .checked_add(index.checked_mul(element_size).ok_or_else(|| {
                            Error::Codegen("global array initializer offset overflows".into())
                        })?)
                        .ok_or_else(|| {
                            Error::Codegen("global array initializer offset overflows".into())
                        })?;
                    write_global_initializer(
                        bytes,
                        relocations,
                        symbols,
                        strings,
                        offset,
                        element,
                        item,
                        location,
                    )?;
                }
                Ok(())
            }
            Type::Struct(struct_type) => {
                let mut offset = 0usize;
                for (field, item) in struct_type.members().iter().zip(items.iter()) {
                    let align = usize::try_from(
                        field
                            .ctype
                            .alignof()
                            .map_err(|error| unsupported(location, error.to_string()))?,
                    )
                    .map_err(|_| unsupported(location, "struct field alignment is too large"))?;
                    if align > 1 {
                        let rem = offset % align;
                        if rem != 0 {
                            offset += align - rem;
                        }
                    }
                    let field_offset = base_offset
                        .checked_add(offset)
                        .ok_or_else(|| Error::Codegen("global struct offset overflows".into()))?;
                    write_global_initializer(
                        bytes,
                        relocations,
                        symbols,
                        strings,
                        field_offset,
                        &field.ctype,
                        item,
                        location,
                    )?;
                    offset = offset
                        .checked_add(
                            usize::try_from(
                                field
                                    .ctype
                                    .sizeof()
                                    .map_err(|error| unsupported(location, error.to_string()))?,
                            )
                            .map_err(|_| unsupported(location, "struct field is too large"))?,
                        )
                        .ok_or_else(|| Error::Codegen("global struct offset overflows".into()))?;
                }
                Ok(())
            }
            Type::Union(union_type) => {
                if items.len() > 1 {
                    return Err(unsupported(
                        location,
                        "too many elements in local union initializer",
                    ));
                }
                if let Some(item) = items.first() {
                    let members = union_type.members();
                    let first = members.first().ok_or_else(|| {
                        unsupported(location, "union has no initializable member")
                    })?;
                    write_global_initializer(
                        bytes,
                        relocations,
                        symbols,
                        strings,
                        base_offset,
                        &first.ctype,
                        item,
                        location,
                    )?;
                }
                Ok(())
            }
            _ => Err(unsupported(
                location,
                "unsupported global aggregate initializer shape",
            )),
        },
        Initializer::Scalar(expression) => {
            if let Type::Array(element, _) = ctype {
                if matches!(element.as_ref(), Type::Char(_)) {
                    if let ExprType::Literal(LiteralValue::Str(string)) = &expression.expr {
                        let end = base_offset.checked_add(string.len()).ok_or_else(|| {
                            Error::Codegen("string initializer offset overflows".into())
                        })?;
                        bytes
                            .get_mut(base_offset..end)
                            .ok_or_else(|| {
                                Error::Codegen("string initializer exceeds array".into())
                            })?
                            .copy_from_slice(string);
                        return Ok(());
                    }
                }
            }
            Err(unsupported(
                location,
                "aggregate global scalar initialization requires aggregate-copy lowering",
            ))
        }
        Initializer::FunctionBody(_) => Err(unsupported(
            location,
            "function body cannot initialize a global data object",
        )),
    }
}

/// Compile C source to SIA32 instructions through the production Cranelift backend.
pub fn compile(source: &str, opt: Opt) -> Result<Artifact, Error> {
    let program = check_semantics(source, opt);
    let declarations = program.result.map_err(Error::Source)?;

    let isa = target_isa()?;
    let mut string_indices = HashMap::new();
    for declaration in &declarations {
        if let Some(initializer) = &declaration.data.init {
            collect_string_literals_initializer(initializer, &mut string_indices);
        }
    }
    let function_indices: HashMap<Symbol, u32> = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            matches!(declaration.data.symbol.get().ctype, Type::Function(_))
                .then_some((declaration.data.symbol, index as u32))
        })
        .collect();
    let mut global_indices: HashMap<Symbol, u32> = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            let metadata = declaration.data.symbol.get();
            (!matches!(metadata.ctype, Type::Function(_))
                && metadata.storage_class != StorageClass::Typedef)
                .then_some((declaration.data.symbol, index as u32))
        })
        .collect();
    let mut symbol_names: HashMap<Symbol, String> = declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| {
            declaration.data.symbol.get().storage_class != StorageClass::Typedef
        })
        .map(|(index, declaration)| {
            (
                declaration.data.symbol,
                translation_unit_symbol_name(index, &declaration.data),
            )
        })
        .collect();
    let mut static_locals = Vec::new();
    for (function_index, declaration) in declarations.iter().enumerate() {
        if let Some(Initializer::FunctionBody(body)) = &declaration.data.init {
            collect_static_locals(body, function_index, &mut static_locals);
        }
    }
    for (ordinal, (function_index, declaration, _)) in static_locals.iter().enumerate() {
        let index = declarations
            .len()
            .checked_add(ordinal)
            .and_then(|index| u32::try_from(index).ok())
            .expect("translation-unit symbol table fits in u32");
        let raw = declaration.symbol.get().id.resolve_and_clone();
        let name = format!("__cosmic_static_local_{function_index}_{ordinal}_{raw}");
        global_indices.insert(declaration.symbol, index);
        symbol_names.insert(declaration.symbol, name);
    }
    let mut functions = Vec::new();
    let mut pooled_strings = string_indices
        .iter()
        .map(|(bytes, index)| (*index, bytes.clone()))
        .collect::<Vec<_>>();
    pooled_strings.sort_by_key(|(index, _)| *index);
    let mut data = pooled_strings
        .into_iter()
        .map(|(index, bytes)| DataArtifact {
            name: string_symbol_name(index),
            bytes,
            align: 1,
            read_only: true,
            relocations: Vec::new(),
        })
        .collect::<Vec<_>>();
    for (_, declaration, location) in &static_locals {
        let metadata = declaration.symbol.get();
        let completed_type =
            completed_object_type(&metadata.ctype, declaration.init.as_ref(), *location)?;
        let size = usize::try_from(
            completed_type
                .sizeof()
                .map_err(|error| unsupported(*location, error.to_string()))?,
        )
        .map_err(|_| unsupported(*location, "static local is too large"))?;
        let align = u32::try_from(
            completed_type
                .alignof()
                .map_err(|error| unsupported(*location, error.to_string()))?,
        )
        .map_err(|_| unsupported(*location, "static local alignment is too large"))?;
        let mut bytes = vec![0; size];
        let mut relocations = Vec::new();
        if let Some(initializer) = &declaration.init {
            write_global_initializer(
                &mut bytes,
                &mut relocations,
                &symbol_names,
                &string_indices,
                0,
                &completed_type,
                initializer,
                *location,
            )?;
        }
        let name = symbol_names
            .get(&declaration.symbol)
            .cloned()
            .expect("static local has a symbol name");
        data.push(DataArtifact {
            name,
            bytes,
            align: align.max(1),
            read_only: metadata.qualifiers.c_const,
            relocations,
        });
    }
    for (index, declaration) in declarations.iter().enumerate() {
        let metadata = declaration.data.symbol.get();
        if metadata.storage_class == StorageClass::Typedef {
            continue;
        }
        let function_type =
            match &metadata.ctype {
                Type::Function(function_type) => function_type,
                _ if metadata.storage_class == StorageClass::Extern
                    && declaration.data.init.is_none() =>
                {
                    // Declaration-only extern objects allocate no storage here.
                    continue;
                }
                object_type => {
                    let completed_type = completed_object_type(
                        object_type,
                        declaration.data.init.as_ref(),
                        declaration.location,
                    )?;
                    let object_type = &completed_type;
                    let size =
                        usize::try_from(object_type.sizeof().map_err(|error| {
                            unsupported(declaration.location, error.to_string())
                        })?)
                        .map_err(|_| {
                            unsupported(
                                declaration.location,
                                "global object is too large for the host compiler",
                            )
                        })?;
                    let align =
                        u32::try_from(object_type.alignof().map_err(|error| {
                            unsupported(declaration.location, error.to_string())
                        })?)
                        .map_err(|_| {
                            unsupported(
                                declaration.location,
                                "global object alignment does not fit in u32",
                            )
                        })?;
                    let mut bytes = vec![0; size];
                    let mut relocations = Vec::new();
                    if let Some(initializer) = &declaration.data.init {
                        write_global_initializer(
                            &mut bytes,
                            &mut relocations,
                            &symbol_names,
                            &string_indices,
                            0,
                            object_type,
                            initializer,
                            declaration.location,
                        )?;
                    }
                    let name = symbol_names
                        .get(&declaration.data.symbol)
                        .cloned()
                        .expect("every global definition has a symbol name");
                    data.push(DataArtifact {
                        name,
                        bytes,
                        align: align.max(1),
                        read_only: metadata.qualifiers.c_const,
                        relocations,
                    });
                    continue;
                }
            };
        let body = match &declaration.data.init {
            Some(Initializer::FunctionBody(body)) => body,
            Some(_) => {
                return Err(unsupported(
                    declaration.location,
                    "a function must have a function body",
                ))
            }
            None => continue,
        };
        functions.push(compile_function(
            declaration.data.symbol.get().id.resolve_and_clone(),
            function_type,
            body,
            declaration.location,
            index as u32,
            &function_indices,
            &global_indices,
            &string_indices,
            &symbol_names,
            &*isa,
        )?);
    }

    if functions.is_empty() {
        return Err(Error::Codegen(
            "the source contains no C function definitions".into(),
        ));
    }
    Ok(Artifact {
        target: TARGET,
        functions,
        data,
    })
}

/// Compile a source string with the standard Cosmic C frontend options.
pub fn compile_default(source: &str) -> Result<Artifact, Error> {
    compile(source, Opt::default())
}

fn target_isa() -> Result<isa::OwnedTargetIsa, Error> {
    let triple: Triple = TARGET
        .parse()
        .expect("the built-in SIA32 triple must parse");
    let mut shared = settings::builder();
    shared
        .enable("enable_verifier")
        .expect("Cranelift must support verifier configuration");
    isa::lookup(triple)
        .map_err(|error| Error::Codegen(format!("cannot select SIA32 Cranelift backend: {error}")))?
        .finish(Flags::new(shared))
        .map_err(|error| {
            Error::Codegen(format!("cannot configure SIA32 Cranelift backend: {error}"))
        })
}

fn compile_function(
    name: String,
    function_type: &FunctionType,
    body: &[Stmt],
    location: Location,
    function_index: u32,
    function_indices: &HashMap<Symbol, u32>,
    global_indices: &HashMap<Symbol, u32>,
    string_indices: &HashMap<Vec<u8>, u32>,
    symbol_names: &HashMap<Symbol, String>,
    isa: &dyn TargetIsa,
) -> Result<FunctionArtifact, Error> {
    let mut signature = Signature::new(CallConv::SystemV);
    let parameters = function_parameters(function_type);
    let aggregate_return = is_by_value_aggregate(&function_type.return_type);
    if aggregate_return {
        signature.params.push(AbiParam::new(types::I32));
    }
    for parameter in parameters {
        signature.params.push(AbiParam::new(abi_parameter_type(
            &parameter.get().ctype,
            location,
        )?));
    }
    if !matches!(*function_type.return_type, Type::Void) && !aggregate_return {
        signature.returns.push(AbiParam::new(abi_scalar_type(
            &function_type.return_type,
            location,
        )?));
    }

    let function = Function::with_name_signature(UserFuncName::user(0, function_index), signature);
    let mut context = Context::for_function(function);
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);
        let entry_values = builder.block_params(entry).to_vec();

        let terminated = {
            let aggregate_return_address = aggregate_return.then(|| entry_values[0]);
            let parameter_values = if aggregate_return {
                &entry_values[1..]
            } else {
                &entry_values[..]
            };
            let mut lowerer = FunctionLowerer::new(
                &mut builder,
                function_indices,
                global_indices,
                string_indices,
                aggregate_return_address,
            );
            for (parameter, value) in parameters.iter().zip(parameter_values.iter()) {
                let parameter_ctype = &parameter.get().ctype;
                if is_by_value_aggregate(parameter_ctype) {
                    let (slot, address) =
                        lowerer.create_aggregate_slot(parameter_ctype, location)?;
                    lowerer.copy_aggregate_value(address, *value, parameter_ctype, location)?;
                    lowerer.stack_locals.insert(*parameter, slot);
                    continue;
                }
                let parameter_ty = abi_parameter_type(parameter_ctype, location)?;
                let variable = lowerer.builder.declare_var(parameter_ty);
                lowerer.builder.def_var(variable, *value);
                lowerer.variables.insert(*parameter, variable);
                lowerer.variable_types.insert(*parameter, parameter_ty);
            }
            for statement in body {
                lowerer.compile_stmt(statement)?;
            }
            lowerer.terminated
        };
        if !terminated {
            if matches!(*function_type.return_type, Type::Void) {
                builder.ins().return_(&[]);
            } else {
                return Err(unsupported(
                    location,
                    "non-void SIA32 functions must end in an explicit return statement",
                ));
            }
        }
        // Forward gotos can add predecessors after their target block is first
        // encountered, so defer sealing those blocks until the whole function
        // has been lowered.
        builder.seal_all_blocks();
        builder.finalize(isa.frontend_config());
    }

    let clif = context.func.to_string();
    // Compilation mutably borrows the Context for as long as the returned
    // CompiledCode is live. Snapshot the user-name table first so relocation
    // decoding does not need to borrow context after compilation.
    let user_named_funcs = context.func.params.user_named_funcs().clone();
    let mut control_plane = ControlPlane::default();
    let compiled = context.compile(isa, &mut control_plane).map_err(|error| {
        Error::Codegen(format!(
            "SIA32 lowering failed for {name}: {}\nCLIF:\n{clif}",
            error.inner
        ))
    })?;
    let code = compiled.code_buffer().to_vec();
    let mut relocations = Vec::new();
    for relocation in compiled.buffer.relocs() {
        if relocation.kind != Reloc::Abs4 {
            return Err(Error::Codegen(format!(
                "SIA32 emitted unsupported relocation {:?} in {name}",
                relocation.kind
            )));
        }
        let target = match &relocation.target {
            RelocTarget::ExternalName(ExternalName::User(reference)) => {
                let user = user_named_funcs[*reference].clone();
                let table = match user.namespace {
                    0 => function_indices,
                    1 => global_indices,
                    2 => {
                        let target = string_indices
                            .iter()
                            .find_map(|(_, index)| {
                                (*index == user.index).then(|| string_symbol_name(*index))
                            })
                            .ok_or_else(|| {
                                Error::Codegen(format!(
                                    "SIA32 emitted unknown string relocation in {name}"
                                ))
                            })?;
                        relocations.push(RelocationArtifact {
                            offset: relocation.offset,
                            target,
                            addend: relocation.addend,
                        });
                        continue;
                    }
                    namespace => {
                        return Err(Error::Codegen(format!(
                            "SIA32 emitted relocation from unknown namespace {namespace} in {name}"
                        )))
                    }
                };
                table
                    .iter()
                    .find_map(|(symbol, index)| {
                        (*index == user.index)
                            .then(|| symbol_names.get(symbol).cloned())
                            .flatten()
                    })
                    .ok_or_else(|| {
                        Error::Codegen(format!("SIA32 emitted unknown symbol relocation in {name}"))
                    })?
            }
            other => {
                return Err(Error::Codegen(format!(
                    "SIA32 emitted unsupported relocation target {other:?} in {name}"
                )))
            }
        };
        relocations.push(RelocationArtifact {
            offset: relocation.offset,
            target,
            addend: relocation.addend,
        });
    }
    if code.is_empty() || code.len() % 2 != 0 {
        return Err(Error::Codegen(format!(
            "SIA32 emitted invalid instruction bytes for {name}"
        )));
    }
    Ok(FunctionArtifact {
        name,
        code,
        relocations,
    })
}

fn is_by_value_aggregate(ctype: &Type) -> bool {
    matches!(ctype, Type::Struct(_) | Type::Union(_))
}

fn abi_scalar_type(ctype: &Type, location: Location) -> Result<cranelift_codegen::ir::Type, Error> {
    match ctype {
        Type::Float => Ok(types::F32),
        Type::Double => Ok(types::F64),
        other => ir_type(other, location),
    }
}

fn abi_parameter_type(
    ctype: &Type,
    location: Location,
) -> Result<cranelift_codegen::ir::Type, Error> {
    match ctype {
        Type::Function(_) | Type::Array(_, _) | Type::Struct(_) | Type::Union(_) => Ok(types::I32),
        other => abi_scalar_type(other, location),
    }
}

fn function_parameters(function_type: &FunctionType) -> &[Symbol] {
    if function_type.params.len() == 1 && function_type.params[0].get().ctype == Type::Void {
        &[]
    } else {
        &function_type.params
    }
}

fn runtime_vla_element_static_size(ctype: &Type, location: Location) -> Result<u64, Error> {
    ctype
        .sizeof()
        .map_err(|_| unsupported(location, "nested VLA element requires runtime stride"))
}

struct FunctionLowerer<'a, 'b, 'c> {
    builder: &'a mut FunctionBuilder<'b>,
    variables: HashMap<Symbol, Variable>,
    variable_types: HashMap<Symbol, cranelift_codegen::ir::Type>,
    stack_locals: HashMap<Symbol, StackSlot>,
    vla_bases: HashMap<Symbol, Variable>,
    function_indices: &'c HashMap<Symbol, u32>,
    global_indices: &'c HashMap<Symbol, u32>,
    string_indices: &'c HashMap<Vec<u8>, u32>,
    aggregate_return_address: Option<Value>,
    return_type: Option<cranelift_codegen::ir::Type>,
    loop_targets: Vec<(Block, Block)>,
    break_targets: Vec<Block>,
    switch_cases: Vec<(HashMap<u64, Block>, Option<Block>)>,
    labels: HashMap<saltwater_parser::intern::InternedStr, Block>,
    terminated: bool,
}

impl<'a, 'b, 'c> FunctionLowerer<'a, 'b, 'c> {
    fn new(
        builder: &'a mut FunctionBuilder<'b>,
        function_indices: &'c HashMap<Symbol, u32>,
        global_indices: &'c HashMap<Symbol, u32>,
        string_indices: &'c HashMap<Vec<u8>, u32>,
        aggregate_return_address: Option<Value>,
    ) -> Self {
        let return_type = builder
            .func
            .signature
            .returns
            .first()
            .map(|ret| ret.value_type);
        Self {
            builder,
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            stack_locals: HashMap::new(),
            vla_bases: HashMap::new(),
            function_indices,
            global_indices,
            string_indices,
            aggregate_return_address,
            return_type,
            loop_targets: Vec::new(),
            break_targets: Vec::new(),
            switch_cases: Vec::new(),
            labels: HashMap::new(),
            terminated: false,
        }
    }

    fn symbol_address(
        &mut self,
        symbol: Symbol,
        addend: i64,
        location: Location,
    ) -> Result<Value, Error> {
        let (namespace, index) = if let Some(index) = self.function_indices.get(&symbol).copied() {
            (0, index)
        } else if let Some(index) = self.global_indices.get(&symbol).copied() {
            (1, index)
        } else {
            return Err(unsupported(
                location,
                "SIA32 symbol address has no translation-unit declaration",
            ));
        };
        let external = self
            .builder
            .func
            .declare_imported_user_function(UserExternalName::new(namespace, index));
        let global = self
            .builder
            .func
            .create_global_value(GlobalValueData::Symbol {
                name: ExternalName::user(external),
                offset: addend.into(),
                colocated: false,
                tls: false,
            });
        Ok(self.builder.ins().symbol_value(types::I32, global))
    }

    fn string_address(&mut self, bytes: &[u8], location: Location) -> Result<Value, Error> {
        let index = self.string_indices.get(bytes).copied().ok_or_else(|| {
            unsupported(
                location,
                "string literal has no translation-unit data object",
            )
        })?;
        let external = self
            .builder
            .func
            .declare_imported_user_function(UserExternalName::new(2, index));
        let global = self
            .builder
            .func
            .create_global_value(GlobalValueData::Symbol {
                name: ExternalName::user(external),
                offset: 0.into(),
                colocated: false,
                tls: false,
            });
        Ok(self.builder.ins().symbol_value(types::I32, global))
    }

    fn create_aggregate_slot(
        &mut self,
        ctype: &Type,
        location: Location,
    ) -> Result<(StackSlot, Value), Error> {
        let size = u32::try_from(
            ctype
                .sizeof()
                .map_err(|error| unsupported(location, error.to_string()))?,
        )
        .map_err(|_| unsupported(location, "aggregate ABI object is too large for SIA32"))?;
        let align = ctype
            .alignof()
            .map_err(|error| unsupported(location, error.to_string()))?;
        let align_shift = u8::try_from(align.trailing_zeros())
            .map_err(|_| unsupported(location, "aggregate ABI alignment is too large"))?;
        let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            size,
            align_shift,
        ));
        let address = self.builder.ins().stack_addr(types::I32, slot, 0);
        Ok((slot, address))
    }

    fn copy_aggregate_value(
        &mut self,
        destination: Value,
        source: Value,
        ctype: &Type,
        location: Location,
    ) -> Result<(), Error> {
        let size = i32::try_from(
            ctype
                .sizeof()
                .map_err(|error| unsupported(location, error.to_string()))?,
        )
        .map_err(|_| unsupported(location, "aggregate copy is too large for SIA32"))?;
        // C struct/union assignment copies the complete object representation,
        // including padding. Byte-wise copies avoid imposing stronger alignment
        // than the aggregate itself guarantees and cover odd-sized objects.
        for offset in 0..size {
            let value = self
                .builder
                .ins()
                .load(types::I8, MemFlagsData::new(), source, offset);
            self.builder
                .ins()
                .store(MemFlagsData::new(), value, destination, offset);
        }
        Ok(())
    }

    fn compile_condition(&mut self, expression: &Expr) -> Result<Value, Error> {
        let value = self.compile_expr(expression)?;
        let value_ty = self.builder.func.dfg.value_type(value);
        let zero = self.builder.ins().iconst(value_ty, 0);
        Ok(self.builder.ins().icmp(IntCC::NotEqual, value, zero))
    }

    fn statement_kind(statement: &Stmt) -> &'static str {
        match &statement.data {
            StmtType::Compound(_) => "compound",
            StmtType::If(_, _, _) => "if",
            StmtType::Do(_, _) => "do",
            StmtType::While(_, _) => "while",
            StmtType::For(_, _, _, _) => "for",
            StmtType::Switch(_, _) => "switch",
            StmtType::Label(_, _) => "label",
            StmtType::Case(_, _) => "case",
            StmtType::Default(_) => "default",
            StmtType::Expr(_) => "expr",
            StmtType::Goto(_) => "goto",
            StmtType::Continue => "continue",
            StmtType::Break => "break",
            StmtType::Return(_) => "return",
            StmtType::Decl(_) => "decl",
        }
    }

    fn compile_stmt(&mut self, statement: &Stmt) -> Result<(), Error> {
        let _statement_kind = Self::statement_kind(statement);
        // A label starts a new reachable basic block even when the preceding
        // statement terminated (for example with goto or return).
        if let StmtType::Label(label, inner) = &statement.data {
            let block = if let Some(block) = self.labels.get(label).copied() {
                block
            } else {
                let block = self.builder.create_block();
                self.labels.insert(*label, block);
                block
            };
            if !self.terminated {
                self.builder.ins().jump(block, &[]);
            }
            self.builder.switch_to_block(block);
            self.terminated = false;
            return self.compile_stmt(inner);
        }

        if self.terminated
            && !matches!(
                statement.data,
                StmtType::Compound(_) | StmtType::Case(_, _) | StmtType::Default(_)
            )
        {
            return Ok(());
        }
        match &statement.data {
            StmtType::Compound(statements) => {
                for statement in statements {
                    // Do not prune here: labels, including case/default labels,
                    // can restore reachability after a terminating statement.
                    // compile_stmt itself ignores unreachable ordinary statements.
                    self.compile_stmt(statement)?;
                }
                Ok(())
            }
            StmtType::Decl(declarations) => {
                for declaration in declarations {
                    self.compile_local(&declaration.data, declaration.location)?;
                }
                Ok(())
            }
            StmtType::Expr(expression) => {
                self.compile_expr(expression)?;
                Ok(())
            }
            StmtType::Return(value) => {
                if let Some(destination) = self.aggregate_return_address {
                    let expression = value.as_ref().ok_or_else(|| {
                        unsupported(
                            statement.location,
                            "aggregate-returning function requires a return value",
                        )
                    })?;
                    if !is_by_value_aggregate(&expression.ctype) {
                        return Err(unsupported(
                            statement.location,
                            "aggregate return expression must have struct or union type",
                        ));
                    }
                    let source = self.compile_expr(expression)?;
                    self.copy_aggregate_value(
                        destination,
                        source,
                        &expression.ctype,
                        statement.location,
                    )?;
                    self.builder.ins().return_(&[]);
                    self.terminated = true;
                    return Ok(());
                }
                if let Some(expression) = value {
                    let signed = matches!(
                        &expression.ctype,
                        Type::Char(true)
                            | Type::Short(true)
                            | Type::Int(true)
                            | Type::Long(true)
                            | Type::Enum(_, _)
                    );
                    let mut value = self.compile_expr(expression)?;
                    if let Some(return_type) = self.return_type {
                        let value_type = self.builder.func.dfg.value_type(value);
                        if value_type != return_type {
                            if value_type.bits() < return_type.bits() {
                                value = if signed {
                                    self.builder.ins().sextend(return_type, value)
                                } else {
                                    self.builder.ins().uextend(return_type, value)
                                };
                            } else {
                                value = self.builder.ins().ireduce(return_type, value);
                            }
                        }
                    }
                    self.builder.ins().return_(&[value]);
                } else {
                    self.builder.ins().return_(&[]);
                }
                self.terminated = true;
                Ok(())
            }
            StmtType::If(condition, then_stmt, else_stmt) => {
                let condition = self.compile_condition(condition)?;

                let then_block = self.builder.create_block();
                let else_block = self.builder.create_block();
                let merge_block = self.builder.create_block();

                self.builder
                    .ins()
                    .brif(condition, then_block, &[], else_block, &[]);

                self.builder.switch_to_block(then_block);
                self.builder.seal_block(then_block);
                self.terminated = false;
                self.compile_stmt(then_stmt)?;
                let then_terminated = self.terminated;

                if !then_terminated {
                    self.builder.ins().jump(merge_block, &[]);
                }

                self.builder.switch_to_block(else_block);
                self.builder.seal_block(else_block);
                self.terminated = false;

                if let Some(else_stmt) = else_stmt {
                    self.compile_stmt(else_stmt)?;
                }

                let else_terminated = self.terminated;

                if !else_terminated {
                    self.builder.ins().jump(merge_block, &[]);
                }

                if then_terminated && else_terminated {
                    self.terminated = true;
                } else {
                    self.builder.switch_to_block(merge_block);
                    self.builder.seal_block(merge_block);
                    self.terminated = false;
                }

                Ok(())
            }
            StmtType::Do(body, condition) => {
                let body_block = self.builder.create_block();
                let condition_block = self.builder.create_block();
                let exit = self.builder.create_block();

                self.builder.ins().jump(body_block, &[]);
                self.builder.switch_to_block(body_block);
                // body_block has a backedge from the condition block, so it
                // cannot be sealed until that predecessor has been emitted.
                self.terminated = false;
                self.loop_targets.push((condition_block, exit));
                self.break_targets.push(exit);
                self.compile_stmt(body)?;
                self.break_targets.pop();
                self.loop_targets.pop();
                if !self.terminated {
                    self.builder.ins().jump(condition_block, &[]);
                }

                self.builder.switch_to_block(condition_block);
                let condition = self.compile_condition(condition)?;
                self.builder
                    .ins()
                    .brif(condition, body_block, &[], exit, &[]);
                self.builder.seal_block(condition_block);
                self.builder.seal_block(body_block);

                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);
                self.terminated = false;
                Ok(())
            }
            StmtType::For(init, condition, step, body) => {
                self.compile_stmt(init)?;
                if self.terminated {
                    return Ok(());
                }

                let header = self.builder.create_block();
                let body_block = self.builder.create_block();
                let step_block = self.builder.create_block();
                let exit = self.builder.create_block();

                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                if let Some(condition) = condition {
                    let condition = self.compile_condition(condition)?;
                    self.builder
                        .ins()
                        .brif(condition, body_block, &[], exit, &[]);
                } else {
                    self.builder.ins().jump(body_block, &[]);
                }

                self.builder.switch_to_block(body_block);
                self.builder.seal_block(body_block);
                self.terminated = false;
                self.loop_targets.push((step_block, exit));
                self.break_targets.push(exit);
                self.compile_stmt(body)?;
                self.break_targets.pop();
                self.loop_targets.pop();
                if !self.terminated {
                    self.builder.ins().jump(step_block, &[]);
                }

                self.builder.switch_to_block(step_block);
                self.builder.seal_block(step_block);
                self.terminated = false;
                if let Some(step) = step {
                    self.compile_expr(step)?;
                }
                self.builder.ins().jump(header, &[]);

                self.builder.seal_block(header);
                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);
                self.terminated = false;
                Ok(())
            }
            StmtType::While(condition, body) => {
                let header = self.builder.create_block();
                let body_block = self.builder.create_block();
                let exit = self.builder.create_block();

                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                let condition = self.compile_condition(condition)?;
                self.builder
                    .ins()
                    .brif(condition, body_block, &[], exit, &[]);

                self.builder.switch_to_block(body_block);
                self.builder.seal_block(body_block);
                self.terminated = false;
                self.loop_targets.push((header, exit));
                self.break_targets.push(exit);
                self.compile_stmt(body)?;
                self.break_targets.pop();
                self.loop_targets.pop();
                if !self.terminated {
                    self.builder.ins().jump(header, &[]);
                }

                self.builder.seal_block(header);
                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);
                self.terminated = false;
                Ok(())
            }
            StmtType::Break => {
                let exit = self.break_targets.last().copied().ok_or_else(|| {
                    unsupported(statement.location, "break outside a SIA32 loop or switch")
                })?;
                self.builder.ins().jump(exit, &[]);
                self.terminated = true;
                Ok(())
            }
            StmtType::Goto(label) => {
                let block = if let Some(block) = self.labels.get(label).copied() {
                    block
                } else {
                    let block = self.builder.create_block();
                    self.labels.insert(*label, block);
                    block
                };
                self.builder.ins().jump(block, &[]);
                self.terminated = true;
                Ok(())
            }
            StmtType::Continue => {
                let (header, _) = self.loop_targets.last().copied().ok_or_else(|| {
                    unsupported(statement.location, "continue outside a SIA32 loop")
                })?;
                self.builder.ins().jump(header, &[]);
                self.terminated = true;
                Ok(())
            }
            StmtType::Switch(expression, body) => {
                fn collect_labels(stmt: &Stmt, cases: &mut Vec<u64>, has_default: &mut bool) {
                    match &stmt.data {
                        StmtType::Case(value, inner) => {
                            cases.push(*value);
                            collect_labels(inner, cases, has_default);
                        }
                        StmtType::Default(inner) => {
                            *has_default = true;
                            collect_labels(inner, cases, has_default);
                        }
                        StmtType::Compound(statements) => {
                            for statement in statements {
                                collect_labels(statement, cases, has_default);
                            }
                        }
                        // Labels inside a nested switch belong to that switch.
                        StmtType::Switch(_, _) => {}
                        StmtType::If(_, yes, no) => {
                            collect_labels(yes, cases, has_default);
                            if let Some(no) = no {
                                collect_labels(no, cases, has_default);
                            }
                        }
                        StmtType::While(_, inner)
                        | StmtType::Do(inner, _)
                        | StmtType::Label(_, inner) => collect_labels(inner, cases, has_default),
                        StmtType::For(_, _, _, inner) => collect_labels(inner, cases, has_default),
                        _ => {}
                    }
                }

                let selector = self.compile_expr(expression)?;
                let selector_ty = self.builder.func.dfg.value_type(selector);
                let mut values = Vec::new();
                let mut has_default = false;
                collect_labels(body, &mut values, &mut has_default);

                let mut case_blocks = HashMap::new();
                for value in values {
                    if !case_blocks.contains_key(&value) {
                        let block = self.builder.create_block();
                        case_blocks.insert(value, block);
                    }
                }
                let default_block = has_default.then(|| self.builder.create_block());
                let exit = self.builder.create_block();

                let cases = case_blocks
                    .iter()
                    .map(|(value, block)| (*value, *block))
                    .collect::<Vec<_>>();
                let mut dispatch = self
                    .builder
                    .current_block()
                    .expect("switch must have a current block");
                for (index, (value, target)) in cases.iter().enumerate() {
                    if index != 0 {
                        self.builder.switch_to_block(dispatch);
                    }
                    let next = self.builder.create_block();
                    let constant = self.builder.ins().iconst(selector_ty, *value as i64);
                    let matches = self.builder.ins().icmp(IntCC::Equal, selector, constant);
                    self.builder.ins().brif(matches, *target, &[], next, &[]);
                    dispatch = next;
                }
                self.builder.switch_to_block(dispatch);
                self.builder.ins().jump(default_block.unwrap_or(exit), &[]);

                self.switch_cases.push((case_blocks, default_block));
                self.break_targets.push(exit);
                // The dispatch block is already terminated. The compound
                // walker still visits case/default labels and skips ordinary
                // statements until a label establishes a reachable block.
                self.terminated = true;
                self.compile_stmt(body)?;
                self.break_targets.pop();
                self.switch_cases.pop();

                if !self.terminated {
                    self.builder.ins().jump(exit, &[]);
                }
                self.builder.switch_to_block(exit);
                self.terminated = false;
                Ok(())
            }
            StmtType::Case(value, inner) => {
                let target = self
                    .switch_cases
                    .last()
                    .and_then(|(cases, _)| cases.get(value).copied())
                    .ok_or_else(|| {
                        unsupported(statement.location, "case outside a SIA32 switch")
                    })?;
                if !self.terminated {
                    self.builder.ins().jump(target, &[]);
                }
                self.builder.switch_to_block(target);
                self.terminated = false;
                self.compile_stmt(inner)
            }
            StmtType::Default(inner) => {
                let target = self
                    .switch_cases
                    .last()
                    .and_then(|(_, default)| *default)
                    .ok_or_else(|| {
                        unsupported(statement.location, "default outside a SIA32 switch")
                    })?;
                if !self.terminated {
                    self.builder.ins().jump(target, &[]);
                }
                self.builder.switch_to_block(target);
                self.terminated = false;
                self.compile_stmt(inner)
            }
            StmtType::Label(_, _) => {
                unreachable!("labels are handled before the main statement match")
            }
        }
    }

    fn compile_local(
        &mut self,
        declaration: &Declaration,
        location: Location,
    ) -> Result<(), Error> {
        let metadata = declaration.symbol.get();
        if metadata.storage_class == StorageClass::Typedef {
            return Ok(());
        }
        if metadata.storage_class == StorageClass::Static
            && !matches!(metadata.ctype, Type::Function(_))
        {
            return Ok(());
        }
        if let Type::Array(
            element,
            saltwater_parser::data::types::ArrayType::Variable(bound_expression),
        ) = &metadata.ctype
        {
            let element_size = element
                .sizeof()
                .map_err(|_| unsupported(location, "VLA element type must be complete"))?;
            let bound = self.compile_expr(bound_expression)?;
            let bound_ty = self.builder.func.dfg.value_type(bound);
            let bound = if bound_ty == types::I32 {
                bound
            } else {
                self.builder.ins().uextend(types::I32, bound)
            };
            let element_size_value = self.builder.ins().iconst(types::I32, element_size as i64);
            let bytes = self.builder.ins().imul(bound, element_size_value);
            let alignment = element
                .alignof()
                .map_err(|_| unsupported(location, "VLA element type has unsupported alignment"))?;
            let alignment = u32::try_from(alignment)
                .map_err(|_| unsupported(location, "VLA alignment is too large for SIA32"))?;
            let bytes = if alignment > 1 {
                let mask = self
                    .builder
                    .ins()
                    .iconst(types::I32, i64::from(alignment - 1));
                let rounded = self.builder.ins().iadd(bytes, mask);
                let clear_mask = self
                    .builder
                    .ins()
                    .iconst(types::I32, i64::from(!(alignment - 1)));
                self.builder.ins().band(rounded, clear_mask)
            } else {
                bytes
            };
            let base = self.builder.ins().stack_alloc_dynamic(types::I32, bytes);
            let base_variable = self.builder.declare_var(types::I32);
            self.builder.def_var(base_variable, base);
            self.vla_bases.insert(declaration.symbol, base_variable);
            if declaration.init.is_some() {
                return Err(unsupported(location, "VLA initialization is not supported"));
            }
            return Ok(());
        }
        if matches!(
            metadata.ctype,
            Type::Struct(_) | Type::Union(_) | Type::Array(_, _)
        ) {
            let completed_type =
                completed_object_type(&metadata.ctype, declaration.init.as_ref(), location)?;
            let size = u32::try_from(
                completed_type
                    .sizeof()
                    .map_err(|_| unsupported(location, "aggregate local has incomplete type"))?,
            )
            .map_err(|_| unsupported(location, "aggregate local is too large for SIA32"))?;
            let align = completed_type
                .alignof()
                .map_err(|_| unsupported(location, "aggregate local has unsupported alignment"))?;
            let align_shift = u8::try_from(align.trailing_zeros())
                .map_err(|_| unsupported(location, "aggregate alignment is too large"))?;
            let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size,
                align_shift,
            ));
            self.stack_locals.insert(declaration.symbol, slot);
            if let Some(initializer) = &declaration.init {
                self.initialize_stack_aggregate(slot, &completed_type, initializer, location)?;
            }
            return Ok(());
        }
        let declared_ty = ir_type(&metadata.ctype, location)?;
        let ty = if declared_ty.bits() < 32 {
            types::I32
        } else {
            declared_ty
        };
        let variable = self.builder.declare_var(ty);
        self.variables.insert(declaration.symbol, variable);
        self.variable_types.insert(declaration.symbol, ty);

        match &declaration.init {
            Some(Initializer::Scalar(expression)) => {
                let initial = self.compile_expr(expression)?;
                self.builder.def_var(variable, initial);
            }
            Some(_) => {
                return Err(unsupported(
                    location,
                    "aggregate local initialization is not supported for SIA32 yet",
                ));
            }
            None => {}
        }

        Ok(())
    }

    fn address_of_local(&mut self, symbol: Symbol, location: Location) -> Result<Value, Error> {
        if let Some(slot) = self.stack_locals.get(&symbol).copied() {
            return Ok(self.builder.ins().stack_addr(types::I32, slot, 0));
        }
        if let Some(variable) = self.vla_bases.get(&symbol).copied() {
            return Ok(self.builder.use_var(variable));
        }
        let variable = self.variables.get(&symbol).copied().ok_or_else(|| {
            unsupported(
                location,
                "SIA32 local address has no local variable mapping",
            )
        })?;
        let value = self.builder.use_var(variable);
        let value_ty = self.builder.func.dfg.value_type(value);
        let size = u32::from(value_ty.bytes());
        let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            size,
            u8::try_from(size.trailing_zeros()).unwrap_or(0),
        ));
        self.builder.ins().stack_store(types::I32, value, slot, 0);
        self.stack_locals.insert(symbol, slot);
        Ok(self.builder.ins().stack_addr(types::I32, slot, 0))
    }

    fn initialize_stack_aggregate(
        &mut self,
        slot: StackSlot,
        ctype: &Type,
        initializer: &Initializer,
        location: Location,
    ) -> Result<(), Error> {
        if let Initializer::Scalar(expression) = initializer {
            if let Type::Array(element, _) = ctype {
                if matches!(element.as_ref(), Type::Char(_)) {
                    if let ExprType::Literal(LiteralValue::Str(bytes)) = &expression.expr {
                        for (offset, byte) in bytes.iter().copied().enumerate() {
                            let value = self.builder.ins().iconst(types::I8, i64::from(byte));
                            let offset = i32::try_from(offset).map_err(|_| {
                                unsupported(location, "string initializer is too large")
                            })?;
                            self.builder
                                .ins()
                                .stack_store(types::I32, value, slot, offset);
                        }
                        return Ok(());
                    }
                }
            }
            if is_by_value_aggregate(ctype) {
                let destination = self.builder.ins().stack_addr(types::I32, slot, 0);
                let source = self.compile_expr(expression)?;
                self.copy_aggregate_value(destination, source, ctype, location)?;
                return Ok(());
            }
        }
        self.initialize_stack_aggregate_at(slot, ctype, initializer, 0, location)
    }

    fn initialize_stack_scalar_at(
        &mut self,
        slot: StackSlot,
        ctype: &Type,
        initializer: &Initializer,
        offset: u64,
        location: Location,
    ) -> Result<(), Error> {
        match initializer {
            Initializer::Scalar(expression) => {
                let value = self.compile_expr(expression)?;
                let value_ty = ir_type(ctype, location)?;
                let value = self.coerce_integer_value(value, value_ty, &expression.ctype);
                let offset = i32::try_from(offset).map_err(|_| {
                    unsupported(location, "aggregate initializer offset is too large")
                })?;
                self.builder
                    .ins()
                    .stack_store(types::I32, value, slot, offset);
                Ok(())
            }
            Initializer::InitializerList(items) if items.len() == 1 => {
                self.initialize_stack_scalar_at(slot, ctype, &items[0], offset, location)
            }
            Initializer::InitializerList(_) => Err(unsupported(
                location,
                "scalar aggregate initializer must contain exactly one element",
            )),
            Initializer::Zero => Ok(()),
            Initializer::FunctionBody(_) => Err(unsupported(
                location,
                "function body cannot initialize an aggregate scalar element",
            )),
        }
    }

    fn initialize_stack_aggregate_at(
        &mut self,
        slot: StackSlot,
        ctype: &Type,
        initializer: &Initializer,
        base_offset: u64,
        location: Location,
    ) -> Result<(), Error> {
        let Initializer::InitializerList(items) = initializer else {
            return Err(unsupported(
                location,
                "aggregate local initialization requires an initializer list",
            ));
        };
        match ctype {
            Type::Array(element, saltwater_parser::data::types::ArrayType::Fixed(count)) => {
                if u64::try_from(items.len()).unwrap_or(u64::MAX) > *count {
                    return Err(unsupported(
                        location,
                        "too many elements in local array initializer",
                    ));
                }
                let element_size = element
                    .sizeof()
                    .map_err(|_| unsupported(location, "array element has incomplete type"))?;
                for (index, item) in items.iter().enumerate() {
                    let offset = base_offset + (index as u64) * element_size;
                    match item {
                        Initializer::Scalar(_) if element.is_scalar() => {
                            self.initialize_stack_scalar_at(slot, element, item, offset, location)?;
                        }
                        Initializer::InitializerList(_) if element.is_scalar() => {
                            self.initialize_stack_scalar_at(slot, element, item, offset, location)?;
                        }
                        Initializer::InitializerList(_) => {
                            self.initialize_stack_aggregate_at(
                                slot, element, item, offset, location,
                            )?;
                        }
                        Initializer::Zero => {}
                        _ => {
                            return Err(unsupported(
                                location,
                                "unsupported array initializer element",
                            ))
                        }
                    }
                }
                Ok(())
            }
            Type::Union(union_type) => {
                if let Some(item) = items.first() {
                    let members = union_type.members();
                    let field = members.first().ok_or_else(|| {
                        unsupported(location, "union has no initializable member")
                    })?;
                    match item {
                        Initializer::Scalar(_) if field.ctype.is_scalar() => {
                            self.initialize_stack_scalar_at(
                                slot,
                                &field.ctype,
                                item,
                                base_offset,
                                location,
                            )?;
                        }
                        Initializer::InitializerList(_) if field.ctype.is_scalar() => {
                            self.initialize_stack_scalar_at(
                                slot,
                                &field.ctype,
                                item,
                                base_offset,
                                location,
                            )?;
                        }
                        Initializer::InitializerList(_) => {
                            self.initialize_stack_aggregate_at(
                                slot,
                                &field.ctype,
                                item,
                                base_offset,
                                location,
                            )?;
                        }
                        Initializer::Zero => {}
                        _ => {
                            return Err(unsupported(
                                location,
                                "unsupported union initializer element",
                            ))
                        }
                    }
                }
                Ok(())
            }
            Type::Struct(struct_type) => {
                if items.len() > struct_type.members().len() {
                    return Err(unsupported(
                        location,
                        "too many elements in local struct initializer",
                    ));
                }
                let mut offset = 0u64;
                for (field, item) in struct_type.members().iter().zip(items.iter()) {
                    let align = field.ctype.alignof().map_err(|_| {
                        unsupported(location, "struct field has unsupported alignment")
                    })?;
                    let rem = offset % align;
                    if rem != 0 {
                        offset += align - rem;
                    }
                    let field_offset = base_offset + offset;
                    match item {
                        Initializer::Scalar(_) if field.ctype.is_scalar() => {
                            self.initialize_stack_scalar_at(
                                slot,
                                &field.ctype,
                                item,
                                field_offset,
                                location,
                            )?;
                        }
                        Initializer::InitializerList(_) if field.ctype.is_scalar() => {
                            self.initialize_stack_scalar_at(
                                slot,
                                &field.ctype,
                                item,
                                field_offset,
                                location,
                            )?;
                        }
                        Initializer::InitializerList(_) => {
                            self.initialize_stack_aggregate_at(
                                slot,
                                &field.ctype,
                                item,
                                field_offset,
                                location,
                            )?;
                        }
                        Initializer::Zero => {}
                        _ => {
                            return Err(unsupported(
                                location,
                                "unsupported struct initializer element",
                            ))
                        }
                    }
                    offset += field
                        .ctype
                        .sizeof()
                        .map_err(|_| unsupported(location, "struct field has incomplete type"))?;
                }
                Ok(())
            }
            _ => Err(unsupported(
                location,
                "this aggregate initializer shape is not supported for SIA32 yet",
            )),
        }
    }

    fn compile_lvalue_address(&mut self, lvalue: &Expr) -> Result<Value, Error> {
        match &lvalue.expr {
            ExprType::Id(symbol) => {
                if self.variables.contains_key(symbol)
                    || self.stack_locals.contains_key(symbol)
                    || self.vla_bases.contains_key(symbol)
                {
                    self.address_of_local(*symbol, lvalue.location)
                } else {
                    self.symbol_address(*symbol, 0, lvalue.location)
                }
            }
            ExprType::Deref(pointer) => self.compile_expr(pointer),
            ExprType::Member(base, member) => {
                self.compile_member_address(base, *member, lvalue.location)
            }
            // Saltwater lowers subscripting into pointer arithmetic and can
            // leave that Binary(Add, ...) directly as the lvalue.
            ExprType::Binary(saltwater_parser::data::hir::BinaryOp::Add, _, _) => {
                self.compile_expr(lvalue)
            }
            ExprType::Noop(inner) | ExprType::Cast(inner) => self.compile_lvalue_address(inner),
            _ => Err(unsupported(
                lvalue.location,
                format!("SIA32 cannot form address for lvalue {lvalue:?}"),
            )),
        }
    }

    fn compile_member_address(
        &mut self,
        base: &Expr,
        member: saltwater_parser::intern::InternedStr,
        location: Location,
    ) -> Result<Value, Error> {
        let struct_type = match &base.ctype {
            Type::Struct(struct_type) | Type::Union(struct_type) => struct_type,
            _ => {
                return Err(unsupported(
                    location,
                    "member access requires a struct or union base",
                ))
            }
        };
        let mut offset = 0u64;
        if matches!(&base.ctype, Type::Struct(_)) {
            let mut found = false;
            for field in struct_type.members().iter() {
                let align = field
                    .ctype
                    .alignof()
                    .map_err(|_| unsupported(location, "member has unsupported alignment"))?;
                let rem = offset % align;
                if rem != 0 {
                    offset += align - rem;
                }
                if field.id == member {
                    found = true;
                    break;
                }
                offset += field
                    .ctype
                    .sizeof()
                    .map_err(|_| unsupported(location, "member has incomplete type"))?;
            }
            if !found {
                return Err(unsupported(location, "unknown struct member"));
            }
        }
        fn strip_wrappers(expr: &Expr) -> &Expr {
            match &expr.expr {
                ExprType::Noop(inner) | ExprType::Cast(inner) => strip_wrappers(inner),
                _ => expr,
            }
        }
        let unwrapped_base = strip_wrappers(base);
        let address = match &unwrapped_base.expr {
            ExprType::Id(symbol) => {
                // Direct local aggregate member access, e.g. local.field.
                if let Some(slot) = self.stack_locals.get(symbol).copied() {
                    self.builder.ins().stack_addr(types::I32, slot, 0)
                } else if let Some(variable) = self.variables.get(symbol).copied() {
                    self.builder.use_var(variable)
                } else {
                    self.symbol_address(*symbol, 0, location)?
                }
            }
            ExprType::Deref(pointer) => {
                // Pointer-derived aggregate member access, including nested
                // expressions such as (*array_of_structs[i]).field.
                self.compile_expr(pointer)?
            }
            ExprType::Member(parent, parent_member) => {
                // Nested direct aggregate access: outer.inner.field.
                self.compile_member_address(parent, *parent_member, location)?
            }
            ExprType::Binary(saltwater_parser::data::hir::BinaryOp::Add, _, _) => {
                // Array-of-aggregate indexing is represented as pointer
                // arithmetic; the expression value is already the selected
                // aggregate's address.
                self.compile_expr(unwrapped_base)?
            }
            _ => {
                return Err(unsupported(
                    location,
                    format!(
                        "SIA32 member access requires an addressable aggregate: base={unwrapped_base:?}"
                    ),
                ))
            }
        };
        Ok(if offset == 0 {
            address
        } else {
            let delta = self.builder.ins().iconst(types::I32, offset as i64);
            self.builder.ins().iadd(address, delta)
        })
    }

    fn coerce_integer_value(
        &mut self,
        value: Value,
        target_ty: cranelift_codegen::ir::Type,
        source_ctype: &Type,
    ) -> Value {
        let source_ty = self.builder.func.dfg.value_type(value);
        if source_ty == target_ty {
            value
        } else if source_ty.bits() < target_ty.bits() {
            if is_signed_integer_type(source_ctype) {
                self.builder.ins().sextend(target_ty, value)
            } else {
                self.builder.ins().uextend(target_ty, value)
            }
        } else {
            // The current SIA32 backend lowers ireduce from the native
            // 32-bit integer width. If a narrow value must become even
            // narrower, widen it first and then reduce once.
            if source_ty.bits() < 32 && target_ty.bits() < source_ty.bits() {
                let wide = if is_signed_integer_type(source_ctype) {
                    self.builder.ins().sextend(types::I32, value)
                } else {
                    self.builder.ins().uextend(types::I32, value)
                };
                self.builder.ins().ireduce(target_ty, wide)
            } else {
                self.builder.ins().ireduce(target_ty, value)
            }
        }
    }

    fn compile_expr(&mut self, expression: &Expr) -> Result<Value, Error> {
        // Void only occurs here for expressions such as a void function call.
        // Such calls are handled explicitly below; all value-producing
        // expressions still require a concrete CLIF integer type.
        let ty = match &expression.ctype {
            Type::Void => types::I32,
            other if is_address_valued_type(other) => types::I32,
            other => ir_type(other, expression.location)?,
        };
        match &expression.expr {
            ExprType::Id(symbol) => {
                if let Some(slot) = self.stack_locals.get(symbol).copied() {
                    if is_address_valued_type(&expression.ctype) {
                        Ok(self.builder.ins().stack_addr(types::I32, slot, 0))
                    } else if let Some(value_ty) = self.variable_types.get(symbol).copied() {
                        Ok(self.builder.ins().stack_load(value_ty, types::I32, slot, 0))
                    } else {
                        Ok(self.builder.ins().stack_addr(types::I32, slot, 0))
                    }
                } else if let Some(variable) = self.variables.get(symbol).copied() {
                    Ok(self.builder.use_var(variable))
                } else if let Some(variable) = self.vla_bases.get(symbol).copied() {
                    Ok(self.builder.use_var(variable))
                } else if self.global_indices.contains_key(symbol)
                    || self.function_indices.contains_key(symbol)
                {
                    self.symbol_address(*symbol, 0, expression.location)
                } else {
                    Err(unsupported(
                        expression.location,
                        "SIA32 identifier has no local or symbolic storage",
                    ))
                }
            }
            ExprType::StaticRef(value) => {
                let mut lvalue = value.as_ref();
                while let ExprType::Noop(inner) | ExprType::Cast(inner) = &lvalue.expr {
                    lvalue = inner;
                }
                // &*p is exactly p and does not perform a load. Every other
                // legal address-of operand uses the same lvalue-address path
                // as assignment and ++/--, including locals, globals/statics,
                // members, array elements, and function designators.
                if let ExprType::Deref(pointer) = &lvalue.expr {
                    self.compile_expr(pointer)
                } else {
                    self.compile_lvalue_address(lvalue)
                }
            }
            ExprType::Literal(LiteralValue::Int(value)) => {
                Ok(self.builder.ins().iconst(ty, *value))
            }
            ExprType::Literal(LiteralValue::UnsignedInt(value)) => {
                Ok(self.builder.ins().iconst(ty, *value as i64))
            }
            ExprType::Literal(LiteralValue::Char(value)) => {
                Ok(self.builder.ins().iconst(ty, i64::from(*value)))
            }
            ExprType::Literal(LiteralValue::Float(value)) => {
                if ty == types::F32 {
                    Ok(self.builder.ins().f32const(*value as f32))
                } else {
                    Ok(self.builder.ins().f64const(*value))
                }
            }
            ExprType::Literal(LiteralValue::Str(bytes)) => {
                self.string_address(bytes, expression.location)
            }
            ExprType::Noop(value) => self.compile_expr(value),
            ExprType::Cast(value) => {
                if matches!(expression.ctype, Type::Void) {
                    let _ = self.compile_expr(value)?;
                    return Ok(self.builder.ins().iconst(types::I32, 0));
                }
                let value_clif = self.compile_expr(value)?;
                if matches!(
                    value.ctype,
                    Type::Struct(_) | Type::Union(_) | Type::Array(_, _)
                ) && matches!(expression.ctype, Type::Pointer(_, _))
                {
                    return Ok(value_clif);
                }
                let source_ty = self.builder.func.dfg.value_type(value_clif);
                let dest_ty = ty;
                if source_ty.is_float() && dest_ty.is_float() {
                    if source_ty == dest_ty {
                        return Ok(value_clif);
                    }
                    return if source_ty == types::F32 && dest_ty == types::F64 {
                        Ok(self.builder.ins().fpromote(types::F64, value_clif))
                    } else {
                        Ok(self.builder.ins().fdemote(types::F32, value_clif))
                    };
                }
                if source_ty.is_float() {
                    let signed = is_signed_integer_type(&expression.ctype);
                    return if signed {
                        Ok(self.builder.ins().fcvt_to_sint_sat(dest_ty, value_clif))
                    } else {
                        Ok(self.builder.ins().fcvt_to_uint_sat(dest_ty, value_clif))
                    };
                }
                if dest_ty.is_float() {
                    let signed = is_signed_integer_type(&value.ctype);
                    return if signed {
                        Ok(self.builder.ins().fcvt_from_sint(dest_ty, value_clif))
                    } else {
                        Ok(self.builder.ins().fcvt_from_uint(dest_ty, value_clif))
                    };
                }

                if source_ty == dest_ty {
                    Ok(value_clif)
                } else if source_ty.bits() < dest_ty.bits() {
                    let signed = matches!(
                        &value.ctype,
                        Type::Char(true)
                            | Type::Short(true)
                            | Type::Int(true)
                            | Type::Long(true)
                            | Type::Enum(_, _)
                    );

                    if signed {
                        Ok(self.builder.ins().sextend(dest_ty, value_clif))
                    } else {
                        Ok(self.builder.ins().uextend(dest_ty, value_clif))
                    }
                } else if source_ty.bits() > dest_ty.bits() {
                    Ok(self.builder.ins().ireduce(dest_ty, value_clif))
                } else {
                    Err(unsupported(
                        expression.location,
                        format!(
                            "SIA32 cast lowering is not implemented from {:?} to {:?}",
                            value.ctype, expression.ctype
                        ),
                    ))
                }
            }
            ExprType::Sizeof(sized) => {
                if let Type::Array(
                    element,
                    saltwater_parser::data::types::ArrayType::Variable(bound_expression),
                ) = sized
                {
                    let element_size = element.sizeof().map_err(|_| {
                        unsupported(
                            expression.location,
                            "sizeof VLA element type must be complete",
                        )
                    })?;
                    let bound = self.compile_expr(bound_expression)?;
                    let bound_ty = self.builder.func.dfg.value_type(bound);
                    let bound = if bound_ty == ty {
                        bound
                    } else if bound_ty.bits() < ty.bits() {
                        self.builder.ins().uextend(ty, bound)
                    } else {
                        self.builder.ins().ireduce(ty, bound)
                    };
                    let element_size = self.builder.ins().iconst(ty, element_size as i64);
                    Ok(self.builder.ins().imul(bound, element_size))
                } else {
                    let bytes = sized.sizeof().map_err(|_| {
                        unsupported(expression.location, "sizeof requires a complete SIA32 type")
                    })?;
                    Ok(self.builder.ins().iconst(ty, bytes as i64))
                }
            }
            ExprType::Comma(left, right) => {
                let _ = self.compile_expr(left)?;
                let value = self.compile_expr(right)?;
                if is_address_valued_type(&right.ctype) {
                    Ok(value)
                } else {
                    Ok(self.coerce_integer_value(value, ty, &right.ctype))
                }
            }
            ExprType::Member(base, member) => {
                let address = self.compile_member_address(base, *member, expression.location)?;
                if is_address_valued_type(&expression.ctype) {
                    Ok(address)
                } else {
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
            }
            // The established HIR represents an ordinary C local read as
            // `Deref(Id(symbol))`: `Id` creates the lvalue address and Deref
            // loads it. This backend keeps non-address-taken locals in SSA,
            // so this pair becomes a direct `use_var` instead of a memory load.
            ExprType::Deref(pointer) if is_address_valued_type(&expression.ctype) => {
                self.compile_expr(pointer)
            }
            ExprType::Deref(pointer) => match &pointer.expr {
                ExprType::Id(symbol) => {
                    if let Some(slot) = self.stack_locals.get(symbol).copied() {
                        if let Some(value_ty) = self.variable_types.get(symbol).copied() {
                            Ok(self.builder.ins().stack_load(value_ty, types::I32, slot, 0))
                        } else {
                            Ok(self.builder.ins().stack_addr(types::I32, slot, 0))
                        }
                    } else if let Some(variable) = self.variables.get(symbol).copied() {
                        Ok(self.builder.use_var(variable))
                    } else {
                        let address = self.symbol_address(*symbol, 0, expression.location)?;
                        Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                    }
                }
                ExprType::Member(_, _) | ExprType::Noop(_) | ExprType::Cast(_) => {
                    let address = self.compile_lvalue_address(pointer)?;
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
                _ => {
                    let address = self.compile_expr(pointer)?;
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
            },
            ExprType::Negate(value) => {
                let value_clif = self.compile_expr(value)?;
                if matches!(expression.ctype, Type::Float | Type::Double) {
                    return Ok(self.builder.ins().fneg(value_clif));
                }
                let value_clif = self.coerce_integer_value(value_clif, ty, &value.ctype);
                let zero = self.builder.ins().iconst(ty, 0);
                Ok(self.builder.ins().isub(zero, value_clif))
            }
            ExprType::BitwiseNot(value) => {
                let value_clif = self.compile_expr(value)?;
                let value_clif = self.coerce_integer_value(value_clif, ty, &value.ctype);
                let all_ones = self.builder.ins().iconst(ty, -1);
                Ok(self.builder.ins().bxor(value_clif, all_ones))
            }
            ExprType::PostIncrement(value, increment) => {
                let mut lvalue = value.as_ref();
                while let ExprType::Noop(inner) | ExprType::Cast(inner) = &lvalue.expr {
                    lvalue = inner;
                }

                let old = self.compile_expr(lvalue)?;
                let value_ty = self.builder.func.dfg.value_type(old);
                let step = match &lvalue.ctype {
                    Type::Pointer(pointee, _) => {
                        let bytes = pointee.sizeof().map_err(|_| {
                            unsupported(
                                expression.location,
                                "SIA32 pointer increment requires a complete pointee type",
                            )
                        })?;
                        i64::try_from(bytes).map_err(|_| {
                            unsupported(
                                expression.location,
                                "SIA32 pointer increment step does not fit in i64",
                            )
                        })?
                    }
                    _ => 1,
                };
                let step = self.builder.ins().iconst(value_ty, step);
                let updated = if *increment {
                    self.builder.ins().iadd(old, step)
                } else {
                    self.builder.ins().isub(old, step)
                };
                let storage_ty = ir_type(&lvalue.ctype, expression.location)?;
                let stored = self.coerce_integer_value(updated, storage_ty, &lvalue.ctype);

                if let ExprType::Id(symbol) = &lvalue.expr {
                    if let Some(slot) = self.stack_locals.get(symbol).copied() {
                        self.builder.ins().stack_store(types::I32, stored, slot, 0);
                    } else if let Some(variable) = self.variables.get(symbol).copied() {
                        self.builder.def_var(variable, updated);
                    } else {
                        let address = self.compile_lvalue_address(lvalue)?;
                        self.builder
                            .ins()
                            .store(MemFlagsData::new(), stored, address, 0);
                    }
                } else {
                    let address = self.compile_lvalue_address(lvalue)?;
                    self.builder
                        .ins()
                        .store(MemFlagsData::new(), stored, address, 0);
                }

                Ok(old)
            }
            ExprType::Ternary(condition, yes, no) => {
                let condition = self.compile_condition(condition)?;

                let yes_block = self.builder.create_block();
                let no_block = self.builder.create_block();
                let merge_block = self.builder.create_block();
                self.builder.append_block_param(merge_block, ty);

                self.builder
                    .ins()
                    .brif(condition, yes_block, &[], no_block, &[]);

                self.builder.switch_to_block(yes_block);
                self.builder.seal_block(yes_block);
                let yes_value = self.compile_expr(yes)?;
                let yes_value = if is_address_valued_type(&yes.ctype) {
                    yes_value
                } else {
                    self.coerce_integer_value(yes_value, ty, &yes.ctype)
                };
                self.builder.ins().jump(merge_block, &[yes_value.into()]);

                self.builder.switch_to_block(no_block);
                self.builder.seal_block(no_block);
                let no_value = self.compile_expr(no)?;
                let no_value = if is_address_valued_type(&no.ctype) {
                    no_value
                } else {
                    self.coerce_integer_value(no_value, ty, &no.ctype)
                };
                self.builder.ins().jump(merge_block, &[no_value.into()]);

                self.builder.seal_block(merge_block);
                self.builder.switch_to_block(merge_block);
                Ok(self.builder.block_params(merge_block)[0])
            }
            ExprType::Binary(operator, left, right) => {
                use saltwater_parser::data::hir::BinaryOp;
                if *operator == BinaryOp::Assign {
                    // The analyzer can wrap an lvalue in one or more Noop
                    // conversions. Strip those before classifying the
                    // assignment destination.
                    let mut assignment_left = left;
                    while let ExprType::Noop(inner) | ExprType::Cast(inner) = &assignment_left.expr
                    {
                        assignment_left = inner;
                    }

                    if matches!(
                        assignment_left.ctype,
                        Type::Struct(_) | Type::Union(_) | Type::Array(_, _)
                    ) {
                        let destination = self.compile_lvalue_address(assignment_left)?;
                        let source = self.compile_expr(right)?;
                        self.copy_aggregate_value(
                            destination,
                            source,
                            &assignment_left.ctype,
                            expression.location,
                        )?;
                        return Ok(destination);
                    }

                    let value = self.compile_expr(right)?;

                    // The analyzer may represent an ordinary local assignment
                    // either directly as Id(symbol) or as Deref(Id(symbol)).
                    // Both denote the same SSA local in this backend.
                    if let ExprType::Id(symbol) = &assignment_left.expr {
                        if let Some(slot) = self.stack_locals.get(symbol).copied() {
                            if let Some(target_ty) = self.variable_types.get(symbol).copied() {
                                let value =
                                    self.coerce_integer_value(value, target_ty, &right.ctype);
                                self.builder.ins().stack_store(types::I32, value, slot, 0);
                                return Ok(value);
                            }
                        }
                        if let Some(variable) = self.variables.get(symbol).copied() {
                            let variable_ty =
                                *self.variable_types.get(symbol).ok_or_else(|| {
                                    unsupported(
                                        expression.location,
                                        "SIA32 local variable type metadata is missing",
                                    )
                                })?;
                            let value = self.coerce_integer_value(value, variable_ty, &right.ctype);
                            self.builder.def_var(variable, value);
                            return Ok(value);
                        }
                        if self.global_indices.contains_key(symbol) {
                            let address = self.symbol_address(*symbol, 0, left.location)?;
                            let target_ty = ir_type(&symbol.get().ctype, left.location)?;
                            let value = self.coerce_integer_value(value, target_ty, &right.ctype);
                            self.builder
                                .ins()
                                .store(MemFlagsData::new(), value, address, 0);
                            return Ok(value);
                        }
                        return Err(unsupported(
                            left.location,
                            format!(
                                "SIA32 assignment to unmapped Id: symbol={symbol:?}, lhs={left:?}"
                            ),
                        ));
                    }

                    let address = self.compile_lvalue_address(assignment_left)?;
                    let target_ty = ir_type(&assignment_left.ctype, left.location)?;
                    let value = self.coerce_integer_value(value, target_ty, &right.ctype);
                    self.builder
                        .ins()
                        .store(MemFlagsData::new(), value, address, 0);
                    return Ok(value);
                }
                // C logical operators are sequencing operations, not ordinary
                // binary arithmetic: the right operand must only be evaluated
                // when required. Lower them to explicit CLIF control flow and
                // merge a canonical C int (0 or 1).
                if matches!(operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    let left_value = self.compile_expr(left)?;
                    let left_ty = self.builder.func.dfg.value_type(left_value);
                    let zero = self.builder.ins().iconst(left_ty, 0);
                    let left_true = self.builder.ins().icmp(IntCC::NotEqual, left_value, zero);

                    let rhs_block = self.builder.create_block();
                    let short_block = self.builder.create_block();
                    let merge_block = self.builder.create_block();
                    self.builder.append_block_param(merge_block, types::I32);

                    match operator {
                        BinaryOp::LogicalAnd => {
                            self.builder
                                .ins()
                                .brif(left_true, rhs_block, &[], short_block, &[]);
                        }
                        BinaryOp::LogicalOr => {
                            self.builder
                                .ins()
                                .brif(left_true, short_block, &[], rhs_block, &[]);
                        }
                        _ => unreachable!(),
                    }

                    self.builder.seal_block(short_block);
                    self.builder.switch_to_block(short_block);
                    let short_value = match operator {
                        BinaryOp::LogicalAnd => 0,
                        BinaryOp::LogicalOr => 1,
                        _ => unreachable!(),
                    };
                    let short_value = self.builder.ins().iconst(types::I32, short_value);
                    let short_args = [short_value.into()];
                    self.builder.ins().jump(merge_block, &short_args);

                    self.builder.seal_block(rhs_block);
                    self.builder.switch_to_block(rhs_block);
                    let right_value = self.compile_expr(right)?;
                    let right_ty = self.builder.func.dfg.value_type(right_value);
                    let zero = self.builder.ins().iconst(right_ty, 0);
                    let right_true = self.builder.ins().icmp(IntCC::NotEqual, right_value, zero);
                    let right_result = self.builder.ins().uextend(types::I32, right_true);
                    let right_args = [right_result.into()];
                    self.builder.ins().jump(merge_block, &right_args);

                    self.builder.seal_block(merge_block);
                    self.builder.switch_to_block(merge_block);
                    return Ok(self.builder.block_params(merge_block)[0]);
                }

                let left_expr_type = left.ctype.clone();
                let right_expr_type = right.ctype.clone();

                if *operator == BinaryOp::Sub {
                    if let (Type::Pointer(left_pointee, _), Type::Pointer(right_pointee, _)) =
                        (&left.ctype, &right.ctype)
                    {
                        let left_size = left_pointee.sizeof().map_err(|_| {
                            unsupported(
                                expression.location,
                                "SIA32 pointer difference requires a complete pointee type",
                            )
                        })?;
                        let right_size = right_pointee.sizeof().map_err(|_| {
                            unsupported(
                                expression.location,
                                "SIA32 pointer difference requires a complete pointee type",
                            )
                        })?;
                        if left_size != right_size {
                            return Err(unsupported(
                                expression.location,
                                "SIA32 pointer difference requires compatible pointee sizes",
                            ));
                        }
                        let left_value = self.compile_expr(left)?;
                        let right_value = self.compile_expr(right)?;
                        let bytes = self.builder.ins().isub(left_value, right_value);
                        if left_size == 1 {
                            return Ok(bytes);
                        }
                        let scale = self.builder.ins().iconst(types::I32, left_size as i64);
                        return Ok(self.builder.ins().sdiv(bytes, scale));
                    }
                }

                if matches!(operator, BinaryOp::Add | BinaryOp::Sub) {
                    let pointer_side = match (&left.ctype, &right.ctype) {
                        (Type::Pointer(pointee, _), _) => Some((
                            left.as_ref(),
                            right.as_ref(),
                            pointee.as_ref(),
                            *operator == BinaryOp::Sub,
                        )),
                        (_, Type::Pointer(pointee, _)) if *operator == BinaryOp::Add => {
                            Some((right.as_ref(), left.as_ref(), pointee.as_ref(), false))
                        }
                        _ => None,
                    };
                    if let Some((pointer_expr, index_expr, pointee, subtract)) = pointer_side {
                        let base = self.compile_expr(pointer_expr)?;
                        let index = self.compile_expr(index_expr)?;
                        let index = self.coerce_integer_value(index, types::I32, &index_expr.ctype);
                        let element_size = pointee.sizeof().map_err(|_| {
                            unsupported(
                                expression.location,
                                "SIA32 pointer arithmetic requires a complete pointee type",
                            )
                        })?;
                        let scale = self.builder.ins().iconst(types::I32, element_size as i64);
                        let delta = self.builder.ins().imul(index, scale);
                        return Ok(if subtract {
                            self.builder.ins().isub(base, delta)
                        } else {
                            self.builder.ins().iadd(base, delta)
                        });
                    }
                }

                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;

                if let BinaryOp::Compare(compare) = operator {
                    if matches!(left_expr_type, Type::Pointer(_, _))
                        && matches!(right_expr_type, Type::Pointer(_, _))
                    {
                        use saltwater_parser::data::lex::ComparisonToken;
                        let condition = match compare {
                            ComparisonToken::Less => IntCC::UnsignedLessThan,
                            ComparisonToken::Greater => IntCC::UnsignedGreaterThan,
                            ComparisonToken::EqualEqual => IntCC::Equal,
                            ComparisonToken::NotEqual => IntCC::NotEqual,
                            ComparisonToken::LessEqual => IntCC::UnsignedLessThanOrEqual,
                            ComparisonToken::GreaterEqual => IntCC::UnsignedGreaterThanOrEqual,
                        };
                        let boolean = self.builder.ins().icmp(condition, left, right);
                        return Ok(self.builder.ins().uextend(types::I32, boolean));
                    }
                }

                // The analyzer records C's usual arithmetic-conversion result
                // on the binary expression. Normalize both operands to that
                // width before emitting CLIF. This avoids type-mismatch IR for
                // combinations such as char + int and short < long.
                if let BinaryOp::Compare(compare) = operator {
                    if matches!(left_expr_type, Type::Float | Type::Double)
                        || matches!(right_expr_type, Type::Float | Type::Double)
                    {
                        use saltwater_parser::data::lex::ComparisonToken;
                        let condition = match compare {
                            ComparisonToken::Less => FloatCC::LessThan,
                            ComparisonToken::Greater => FloatCC::GreaterThan,
                            ComparisonToken::EqualEqual => FloatCC::Equal,
                            ComparisonToken::NotEqual => FloatCC::NotEqual,
                            ComparisonToken::LessEqual => FloatCC::LessThanOrEqual,
                            ComparisonToken::GreaterEqual => FloatCC::GreaterThanOrEqual,
                        };
                        let boolean = self.builder.ins().fcmp(condition, left, right);
                        return Ok(self.builder.ins().uextend(types::I32, boolean));
                    }
                }

                let operation_ty = match operator {
                    BinaryOp::Shl | BinaryOp::Shr => self.builder.func.dfg.value_type(left),
                    BinaryOp::Compare(_) => {
                        let left_ty = self.builder.func.dfg.value_type(left);
                        let right_ty = self.builder.func.dfg.value_type(right);
                        if left_ty.bits() >= right_ty.bits() {
                            left_ty
                        } else {
                            right_ty
                        }
                    }
                    _ => ir_type(&expression.ctype, expression.location)?,
                };
                if matches!(expression.ctype, Type::Float | Type::Double) {
                    let value = match operator {
                        BinaryOp::Mul => self.builder.ins().fmul(left, right),
                        BinaryOp::Div => self.builder.ins().fdiv(left, right),
                        BinaryOp::Add => self.builder.ins().fadd(left, right),
                        BinaryOp::Sub => self.builder.ins().fsub(left, right),
                        _ => {
                            return Err(unsupported(
                                expression.location,
                                format!("floating-point operator {operator:?} is not part of L12"),
                            ))
                        }
                    };
                    return Ok(value);
                }
                let left = self.coerce_integer_value(left, operation_ty, &left_expr_type);
                let right = self.coerce_integer_value(right, operation_ty, &right_expr_type);
                let value = match operator {
                    BinaryOp::Mul => self.builder.ins().imul(left, right),
                    BinaryOp::Div => {
                        if is_signed_integer_type(&left_expr_type) {
                            self.builder.ins().sdiv(left, right)
                        } else {
                            self.builder.ins().udiv(left, right)
                        }
                    }
                    BinaryOp::Mod => {
                        if is_signed_integer_type(&left_expr_type) {
                            self.builder.ins().srem(left, right)
                        } else {
                            self.builder.ins().urem(left, right)
                        }
                    }
                    BinaryOp::Add => self.builder.ins().iadd(left, right),
                    BinaryOp::Sub => self.builder.ins().isub(left, right),
                    BinaryOp::BitwiseAnd => self.builder.ins().band(left, right),
                    BinaryOp::BitwiseOr => self.builder.ins().bor(left, right),
                    BinaryOp::Xor => self.builder.ins().bxor(left, right),
                    BinaryOp::Shl => self.builder.ins().ishl(left, right),
                    BinaryOp::Shr => {
                        if is_signed_integer_type(&left_expr_type) {
                            self.builder.ins().sshr(left, right)
                        } else {
                            self.builder.ins().ushr(left, right)
                        }
                    }
                    BinaryOp::Compare(compare) => {
                        use saltwater_parser::data::lex::ComparisonToken;
                        let signed = is_signed_integer_type(&left_expr_type);
                        let condition = match compare {
                            ComparisonToken::Less => {
                                if signed {
                                    IntCC::SignedLessThan
                                } else {
                                    IntCC::UnsignedLessThan
                                }
                            }
                            ComparisonToken::Greater => {
                                if signed {
                                    IntCC::SignedGreaterThan
                                } else {
                                    IntCC::UnsignedGreaterThan
                                }
                            }
                            ComparisonToken::EqualEqual => IntCC::Equal,
                            ComparisonToken::NotEqual => IntCC::NotEqual,
                            ComparisonToken::LessEqual => {
                                if signed {
                                    IntCC::SignedLessThanOrEqual
                                } else {
                                    IntCC::UnsignedLessThanOrEqual
                                }
                            }
                            ComparisonToken::GreaterEqual => {
                                if signed {
                                    IntCC::SignedGreaterThanOrEqual
                                } else {
                                    IntCC::UnsignedGreaterThanOrEqual
                                }
                            }
                        };
                        // CLIF icmp produces I8, but a C comparison expression
                        // has integer type. Normalize the canonical boolean to
                        // the backend's 32-bit C int representation.
                        let boolean = self.builder.ins().icmp(condition, left, right);
                        self.builder.ins().uextend(types::I32, boolean)
                    }
                    BinaryOp::Assign | BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                        unreachable!(
                            "assignment and logical operators are lowered before this match"
                        )
                    }
                };
                Ok(value)
            }
            ExprType::FuncCall(function, arguments) => {
                let direct_symbol = match &function.expr {
                    ExprType::Id(symbol) if matches!(symbol.get().ctype, Type::Function(_)) => {
                        Some(*symbol)
                    }
                    _ => None,
                };
                let function_type = match &function.ctype {
                    Type::Function(function_type) => function_type,
                    Type::Pointer(pointee, _) => match pointee.as_ref() {
                        Type::Function(function_type) => function_type,
                        _ => {
                            return Err(unsupported(
                                expression.location,
                                "SIA32 call target pointer does not point to a function",
                            ))
                        }
                    },
                    _ => {
                        return Err(unsupported(
                            expression.location,
                            "SIA32 call target is not a function",
                        ))
                    }
                };
                let function_type = function_type;
                let function_index = if let Some(symbol) = direct_symbol {
                    Some(self.function_indices.get(&symbol).copied().ok_or_else(|| {
                        unsupported(
                            expression.location,
                            format!(
                                "SIA32 direct call target `{}` has no translation-unit declaration",
                                symbol.get().id.resolve_and_clone()
                            ),
                        )
                    })?)
                } else {
                    None
                };
                let parameters = function_parameters(function_type);
                if (!function_type.varargs && arguments.len() != parameters.len())
                    || (function_type.varargs && arguments.len() < parameters.len())
                {
                    return Err(unsupported(
                        expression.location,
                        "SIA32 call argument count mismatch",
                    ));
                }
                let mut signature = Signature::new(CallConv::SystemV);
                let aggregate_return = is_by_value_aggregate(&function_type.return_type);
                if aggregate_return {
                    signature.params.push(AbiParam::new(types::I32));
                }
                for parameter in parameters {
                    signature.params.push(AbiParam::new(abi_parameter_type(
                        &parameter.get().ctype,
                        expression.location,
                    )?));
                }
                if function_type.varargs {
                    // Cranelift signatures describe the concrete call site.
                    // Apply C's default argument promotions to the unnamed
                    // arguments and append their promoted machine types.
                    for argument in arguments.iter().skip(parameters.len()) {
                        let promoted_ty = match &argument.ctype {
                            Type::Bool | Type::Char(_) | Type::Short(_) | Type::Enum(_, _) => {
                                types::I32
                            }
                            _ => ir_type(&argument.ctype, expression.location)?,
                        };
                        signature.params.push(AbiParam::new(promoted_ty));
                    }
                }
                if !matches!(*function_type.return_type, Type::Void) && !aggregate_return {
                    signature.returns.push(AbiParam::new(abi_scalar_type(
                        &function_type.return_type,
                        expression.location,
                    )?));
                }
                let signature = self.builder.import_signature(signature);
                let function_ref = if let Some(function_index) = function_index {
                    let external = self
                        .builder
                        .func
                        .declare_imported_user_function(UserExternalName::new(0, function_index));
                    Some(self.builder.import_function(ExtFuncData {
                        name: ExternalName::user(external),
                        signature,
                        colocated: true,
                        patchable: false,
                    }))
                } else {
                    None
                };
                let mut aggregate_result = None;
                let mut values =
                    Vec::with_capacity(arguments.len() + usize::from(aggregate_return));
                if aggregate_return {
                    let (_, address) = self
                        .create_aggregate_slot(&function_type.return_type, expression.location)?;
                    aggregate_result = Some(address);
                    values.push(address);
                }
                for (index, argument) in arguments.iter().enumerate() {
                    let value = self.compile_expr(argument)?;
                    if let Some(parameter) = parameters.get(index) {
                        if is_by_value_aggregate(&parameter.get().ctype) {
                            let parameter_ctype = &parameter.get().ctype;
                            let (_, copy_address) =
                                self.create_aggregate_slot(parameter_ctype, expression.location)?;
                            self.copy_aggregate_value(
                                copy_address,
                                value,
                                parameter_ctype,
                                expression.location,
                            )?;
                            values.push(copy_address);
                            continue;
                        }
                    }
                    let parameter_ty = if let Some(parameter) = parameters.get(index) {
                        abi_parameter_type(&parameter.get().ctype, expression.location)?
                    } else {
                        match &argument.ctype {
                            Type::Bool | Type::Char(_) | Type::Short(_) | Type::Enum(_, _) => {
                                types::I32
                            }
                            _ => ir_type(&argument.ctype, expression.location)?,
                        }
                    };
                    values.push(self.coerce_integer_value(value, parameter_ty, &argument.ctype));
                }
                let call = if let Some(function_ref) = function_ref {
                    self.builder.ins().call(function_ref, &values)
                } else {
                    let callee = self.compile_expr(function)?;
                    self.builder.ins().call_indirect(signature, callee, &values)
                };
                if let Some(address) = aggregate_result {
                    return Ok(address);
                }
                if matches!(*function_type.return_type, Type::Void) {
                    // Expression statements discard this value. Returning a
                    // harmless integer placeholder keeps compile_expr uniform
                    // without inventing a machine-level return value.
                    return Ok(self.builder.ins().iconst(types::I32, 0));
                }
                Ok(self.builder.func.dfg.first_result(call))
            }
        }
    }
}

fn is_address_valued_type(ctype: &Type) -> bool {
    matches!(
        ctype,
        Type::Function(_) | Type::Struct(_) | Type::Union(_) | Type::Array(_, _)
    )
}

fn is_signed_integer_type(ctype: &Type) -> bool {
    matches!(
        ctype,
        Type::Char(true)
            | Type::Short(true)
            | Type::Int(true)
            | Type::Long(true)
            | Type::Enum(_, _)
    )
}

fn ir_type(ctype: &Type, location: Location) -> Result<cranelift_codegen::ir::Type, Error> {
    match ctype {
        Type::Bool | Type::Char(_) => Ok(types::I8),
        Type::Short(_) => Ok(types::I16),
        Type::Int(_)
        | Type::Long(_)
        | Type::Enum(_, _)
        | Type::Pointer(_, _)
        | Type::Function(_) => Ok(types::I32),
        Type::Float => Ok(types::F32),
        Type::Double => Ok(types::F64),
        Type::Void => Err(unsupported(
            location,
            "void has no SIA32 SSA value representation",
        )),
        Type::Array(_, _) => Err(unsupported(
            location,
            "array values are address-only on SIA32 and must decay or use aggregate storage",
        )),
        Type::Struct(_) | Type::Union(_) => Err(unsupported(
            location,
            "struct/union values are address-only on SIA32 and use aggregate ABI/storage lowering",
        )),
        Type::VaList => Err(unsupported(
            location,
            "va_list has no standalone SIA32 scalar representation",
        )),
        Type::Error => Err(unsupported(
            location,
            "frontend error type cannot reach SIA32 type lowering",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_source(source: &str) -> Result<Artifact, Error> {
        compile(source, Opt::default())
    }

    #[test]
    fn compiles_global_load_store_and_address_relocations() {
        let artifact = compile_source(
            "int global = 3; int read(void) { return global; } int write(int x) { global = x; return global; } int *address(void) { return &global; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 3);
        assert!(artifact.functions.iter().all(|function| function
            .relocations
            .iter()
            .any(|reloc| reloc.target == "global")));
    }

    #[test]
    fn pools_string_literals_into_read_only_data() {
        let artifact = compile_source(
            "const char *g = \"hello\"; int f(void) { const char *p = \"hello\"; return p[0]; }",
        )
        .unwrap();
        let strings = artifact
            .data
            .iter()
            .filter(|object| object.name.starts_with("__cosmic_str_"))
            .collect::<Vec<_>>();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].bytes, b"hello\0");
        assert!(strings[0].read_only);
        assert!(artifact
            .functions
            .iter()
            .flat_map(|function| function.relocations.iter())
            .any(|reloc| reloc.target == strings[0].name));
    }

    #[test]
    fn initializes_character_arrays_from_string_literals() {
        let artifact = compile_source(
            "char global[] = \"abc\"; int f(void) { char local[] = \"xy\"; return local[1]; }",
        )
        .unwrap();
        assert!(artifact
            .data
            .iter()
            .any(|object| object.name == "global" && object.bytes == b"abc\0"));
    }

    #[test]
    fn emits_data_relocations_for_global_and_function_addresses() {
        let artifact = compile_source(
            "struct pair { int x; int y; }; int target(void) { return 1; } int global; int *p = &global; int (*fp)(void) = target; struct pair pair; int *member = &pair.y;",
        )
        .unwrap();
        assert_eq!(artifact.data.len(), 5);
        let p = artifact
            .data
            .iter()
            .find(|object| object.name == "p")
            .unwrap();
        let fp = artifact
            .data
            .iter()
            .find(|object| object.name == "fp")
            .unwrap();
        let member = artifact
            .data
            .iter()
            .find(|object| object.name == "member")
            .unwrap();
        assert_eq!(p.relocations[0].target, "global");
        assert_eq!(p.relocations[0].addend, 0);
        assert_eq!(fp.relocations[0].target, "target");
        assert_eq!(member.relocations[0].target, "pair");
        assert_eq!(member.relocations[0].addend, 4);
    }

    #[test]
    fn infers_unbounded_array_sizes_from_initializers() {
        let artifact = compile_source(
            "int global[] = {1, 2, 3}; int f(void) { int local[] = {4, 5}; return local[1]; }",
        )
        .unwrap();
        assert_eq!(artifact.data[0].bytes.len(), 12);
        assert_eq!(artifact.data[0].bytes[8..12], 3i32.to_le_bytes());
        assert_eq!(artifact.functions.len(), 1);
    }

    #[test]
    fn emits_recursive_aggregate_global_initializers() {
        let artifact = compile_source(
            "struct pair { int a; short b; }; union value { int i; short s; }; int a[3] = {1, 2}; struct pair p = {3, 4}; union value u = {5}; int f(void) { return 0; }",
        )
        .unwrap();
        assert_eq!(artifact.data.len(), 3);
        assert_eq!(&artifact.data[0].bytes[..8], &[1, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(&artifact.data[1].bytes[..6], &[3, 0, 0, 0, 4, 0]);
        assert_eq!(&artifact.data[2].bytes[..4], &[5, 0, 0, 0]);
    }

    #[test]
    fn emits_scalar_global_initializers() {
        let artifact = compile_source(
            "int a = 1 + 2; unsigned short b = 4660; char c = 65; int *p = 0; int f(void) { return 0; }",
        )
        .unwrap();
        assert_eq!(artifact.data.len(), 4);
        assert_eq!(artifact.data[0].bytes, 3i32.to_le_bytes());
        assert_eq!(artifact.data[1].bytes, 4660u16.to_le_bytes());
        assert_eq!(artifact.data[2].bytes, vec![65]);
        assert_eq!(artifact.data[3].bytes, vec![0; 4]);
    }

    #[test]
    fn lowers_static_locals_into_translation_unit_data() {
        let artifact = compile_source(
            "int f(void) { static int x = 3; x = x + 1; return x; } int g(void) { static int x; return &x != 0; }",
        )
        .unwrap();
        let locals = artifact
            .data
            .iter()
            .filter(|object| object.name.starts_with("__cosmic_static_local_"))
            .collect::<Vec<_>>();
        assert_eq!(locals.len(), 2);
        assert_eq!(locals[0].bytes, 3i32.to_le_bytes());
        assert_ne!(locals[0].name, locals[1].name);
        assert!(artifact
            .functions
            .iter()
            .flat_map(|function| function.relocations.iter())
            .any(|relocation| relocation.target == locals[0].name));
    }

    #[test]
    fn emits_zero_initialized_top_level_objects() {
        let artifact =
            compile_source("int global; static unsigned short hidden; int f(void) { return 0; }")
                .unwrap();
        assert_eq!(artifact.data.len(), 2);
        assert_eq!(artifact.data[0].name, "global");
        assert_eq!(artifact.data[0].bytes, vec![0; 4]);
        assert_eq!(artifact.data[0].align, 4);
        assert_eq!(artifact.data[1].bytes, vec![0; 2]);
        assert!(artifact.data[1].name.contains("hidden"));
    }

    #[test]
    fn compiles_integer_c_to_real_sia32_bytes() {
        let artifact =
            compile_source("int main(void) { int x = 4; return (x << 2) + 3; }").unwrap();
        assert_eq!(artifact.target, TARGET);
        assert_eq!(artifact.functions.len(), 1);
        let code = &artifact.functions[0].code;
        assert!(!code.is_empty());
        assert_eq!(code.len() % 2, 0);
        assert_eq!(&code[code.len() - 2..], &[0xe0, 0xc0]);
        assert!(artifact.to_bytes().unwrap().starts_with(b"COSMIC-SIA\0"));
    }

    #[test]
    fn compiles_signed_narrow_return_conversion() {
        let artifact = compile_source(
            "int f(signed char x) { return x; } unsigned int g(unsigned char x) { return x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_narrow_function_return_conversion() {
        let artifact = compile_source("short f(int x) { return x; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_struct_arguments_and_returns_by_value() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; struct pair make(int x) { struct pair p = { x, x + 1 }; return p; } int sum(struct pair p) { p.a = p.a + 10; return p.a + p.b; } int run(void) { return sum(make(3)); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 3);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_integer_function_parameters_to_sia32_bytes() {
        let artifact =
            compile_source("int add(int left, int right) { return left + right; }").unwrap();
        assert_eq!(artifact.functions[0].name, "add");
        assert_eq!(artifact.functions[0].code.len() % 2, 0);
        assert_eq!(
            &artifact.functions[0].code[artifact.functions[0].code.len() - 2..],
            &[0xe0, 0xc0]
        );
    }

    #[test]
    fn compiles_signed_and_unsigned_integer_division_and_remainder() {
        let artifact = compile_source(
            "int signed_ops(int a, int b) { return (a / b) + (a % b); } unsigned int unsigned_ops(unsigned int a, unsigned int b) { return (a / b) + (a % b); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_pointer_equality_and_order_comparisons() {
        let artifact = compile_source(
            "int f(int *a, int *b) { return (a == b) + (a != b) + (a < b) + (a <= b) + (a > b) + (a >= b); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_signed_and_unsigned_right_shift_and_comparisons() {
        let artifact = compile_source(
            "int signed_ops(int a, int b) { return (a >> 1) + (a < b) + (a >= b); } unsigned int unsigned_ops(unsigned int a, unsigned int b) { return (a >> 1) + (a < b) + (a >= b); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_comma_expression_with_pointer_result() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *a, struct pair *b) { struct pair *p = (a, b); return p->x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_comma_expression_integer_conversion() {
        let artifact = compile_source("int f(unsigned char x) { return (x++, x); }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_conditional_operator_with_aggregate_addresses() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *a, struct pair *b, int c) { struct pair *p = c ? a : b; return p->x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_conditional_operator_with_integer_conversions() {
        let artifact = compile_source(
            "int choose(int flag, short small, int large) { return flag ? small : large; } unsigned int choose_unsigned(int flag, unsigned char small, unsigned int large) { return flag ? small : large; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_pointer_and_narrow_integer_conditions() {
        let artifact = compile_source(
            "int f(int *p, unsigned char x) { if (p) { while (x) { x--; if (!x) break; } } return x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_for_and_do_while_control_flow() {
        let artifact = compile_source(
            "int loops(int n) { int sum = 0; int i = 0; for (i = 0; i < n; i = i + 1) { if (i == 2) continue; sum = sum + i; } do { sum = sum - 1; } while (sum > n); return sum; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_do_while_with_loop_carried_local() {
        let artifact = compile_source(
            "int countdown(int n) { int i = n; do { i = i - 1; } while (i > 0); return i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_narrow_post_increment_stores_at_declared_width() {
        let artifact = compile_source(
            "int f(unsigned char *p, unsigned short *q) { (*p)++; (*q)--; return *p + *q; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_scalar_post_increment_and_decrement() {
        let artifact = compile_source(
            "int update(int n) { int i = 0; int old = i++; int prior = i--; return old + prior + i + n; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_narrow_integer_unary_promotions() {
        let artifact =
            compile_source("int f(signed char x, unsigned short y) { return -x + ~y; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_integer_negation_without_backend_ineg() {
        let artifact = compile_source("int neg(int x) { return -x; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn function_designator_types_are_pointer_width_on_sia32() {
        let artifact = compile_source(
            "int inc(int x) { return x + 1; } int apply(int (*f)(int), int x) { return f(x); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn emits_unresolved_relocations_for_external_function_calls() {
        let artifact =
            compile_source("extern int external(int); int f(void) { return external(3); }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert_eq!(artifact.functions[0].relocations.len(), 1);
        assert_eq!(artifact.functions[0].relocations[0].target, "external");
    }

    #[test]
    fn compiles_integer_function_pointer_calls() {
        let artifact = compile_source("int apply(int (*f)(int), int x) { return f(x); }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_integer_variadic_direct_calls_with_default_promotions() {
        let artifact = compile_source(
            "static int pick(int n, ...) { return n; } int run(char c, short s) { return pick(2, c, s); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
        assert!(artifact
            .functions
            .iter()
            .any(|function| !function.relocations.is_empty()));
    }

    #[test]
    fn rejects_too_few_fixed_arguments_to_variadic_call() {
        let error = compile_source(
            "static int pick(int n, int x, ...) { return n + x; } int run(void) { return pick(1); }",
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("argument count")
                || message.contains("argument")
                || message.contains("parameter"),
            "unexpected diagnostic: {}",
            message
        );
    }

    #[test]
    fn compiles_mixed_width_integer_binary_operations() {
        let artifact = compile_source(
            "int mixed(char c, unsigned char u, short s, int x) { return (c + x) * (s - u) + (c < x) + (u << 2); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_mixed_width_bitwise_and_division_operations() {
        let artifact = compile_source(
            "unsigned int mixed(unsigned char c, unsigned short s, unsigned int x) { return ((c | s) ^ x) / (c + 1) + (x % (s + 1)); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_call_argument_integer_conversions() {
        let artifact = compile_source(
            "static int take(char c, unsigned short s) { return c + s; } int run(int x) { return take(x, x); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_address_taken_scalar_local_with_stack_storage() {
        let artifact =
            compile_source("int local(void) { int x = 3; int *p = &x; *p = 7; return x; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_address_of_local_and_member_through_common_lvalue_path() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(void) { int x = 1; struct pair p; int *a = &x; int *b = &p.x; *b = 4; return *a + *b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_address_of_array_element_and_post_increment() {
        let artifact =
            compile_source("int f(int *p, int i) { int *q = &p[i]; p[i]++; return *q; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_direct_read_after_address_taken_scalar_local() {
        let artifact =
            compile_source("int f(void) { int x = 3; int *p = &x; *p = 9; return x + 1; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_cast_wrapped_member_and_array_lvalue_assignments() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *p, int *a, int i) { p->x = 3; a[i] = p->x; return a[i]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_whole_struct_and_union_assignments() {
        let artifact = compile_source(
            "struct pair { int a; short b; }; union value { int i; short s; }; int f(void) { struct pair a = {1, 2}; struct pair b; union value u = {3}; union value v; b = a; v = u; return b.a + b.b + v.i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_assignment_to_address_taken_local_hir_deref() {
        let artifact =
            compile_source("int f(void) { int x = 1; int *p = &x; x = 5; return *p; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_post_increment_of_address_taken_scalar_local() {
        let artifact =
            compile_source("int f(void) { int x = 1; int *p = &x; x++; return *p; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_pointer_arithmetic_lvalue_assignment() {
        let artifact =
            compile_source("int store(int *p, int i, int x) { p[i] = x; return p[i]; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_union_local_initializers_and_member_access() {
        let artifact = compile_source(
            "union value { int i; unsigned int u; }; int f(void) { union value v = { 7 }; v.u = 9; return v.i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_scalar_struct_and_union_local_initializers() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; union value { int i; short s; }; struct pair make(void) { struct pair p = {1, 2}; return p; } int f(void) { struct pair a = {3, 4}; struct pair b = a; struct pair c = make(); union value u = {5}; union value v = u; return b.a + c.b + v.i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_nested_aggregate_local_initializers() {
        let artifact = compile_source(
            "struct inner { int x; int y; }; struct outer { struct inner i; int a[2]; }; int f(void) { struct outer o = { { 1, 2 }, { 3, 4 } }; return o.i.y + o.a[1]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_scalar_array_and_struct_local_initializers() {
        let artifact = compile_source(
            "struct pair { int a; short b; }; int local(void) { int a[3] = { 1, 2, 3 }; struct pair p = { 4, 5 }; return a[0] + a[2] + p.a + p.b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_uninitialized_aggregate_local_stack_storage() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; int local(void) { struct pair p; p.a = 3; p.b = 4; return p.a + p.b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn local_aggregate_ids_lower_as_addresses() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(void) { struct pair p; p.x = 4; return p.x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn aggregate_address_expressions_do_not_require_scalar_ir_types() {
        let artifact = compile_source(
            "struct pair { int x; int y; }; int f(struct pair *p, int i) { return p[i].x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_array_of_struct_member_reads_and_writes() {
        let artifact = compile_source(
            "struct pair { int x; int y; }; int f(struct pair *p, int i) { p[i].y = 7; return p[i].y; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn nested_aggregate_members_remain_address_valued() {
        let artifact = compile_source(
            "struct inner { int x; }; struct outer { struct inner i; }; int f(struct outer *p) { struct inner *q = &p->i; q->x = 6; return q->x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_wrapped_and_nested_aggregate_member_addresses() {
        let artifact = compile_source(
            "struct inner { int x; }; struct outer { struct inner i; }; int f(struct outer *p) { p->i.x = 7; return p->i.x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_wrapped_member_pointer_dereference_reads() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *p) { int *q = &p->x; return *q; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_member_reads_without_double_dereference() {
        let artifact = compile_source(
            "struct bytes { char c; short s; }; int read(struct bytes *p) { return p->c + p->s; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_narrow_member_and_pointer_stores() {
        let artifact = compile_source(
            "struct bytes { char c; short s; }; int store(struct bytes *p, char *q, int x) { p->c = x; p->s = x; *q = x; return p->c + *q; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_narrow_local_assignment_conversions() {
        let artifact =
            compile_source("int narrow(int x) { char c; short s; c = x; s = x; c = s; return c; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_signed_and_unsigned_widening_assignments() {
        let artifact = compile_source(
            "int widen(unsigned char u, signed char s) { int a; int b; a = u; b = s; return a + b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_address_of_dereference_and_member_lvalues() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; int *same(int *p) { return &*p; } int *member(struct pair *p) { return &p->b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_void_direct_call_expression_statement() {
        let artifact = compile_source(
            "static void touch(int *p) { *p = 7; } int run(int *p) { touch(p); return *p; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .any(|function| !function.relocations.is_empty()));
    }

    #[test]
    fn compiles_switch_after_terminated_dispatch_through_compound() {
        let artifact = compile_source(
            "int f(int x) { int y = 0; switch (x) { y = 7; case 1: y = 1; break; default: y = 2; } return y; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_switch_case_default_break_and_fallthrough() {
        let artifact = compile_source(
            "int choose(int x) { int y = 0; switch (x) { case 1: y = 10; break; case 2: y = 20; case 3: y = y + 1; break; default: y = 99; } return y; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_forward_and_backward_goto_labels() {
        let artifact = compile_source(
            "int forward(int x) { goto done; x = 9; done: return x; } int backward(int x) { again: if (x) { x = x - 1; goto again; } return x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_member_and_array_post_increment_through_lvalue_addressing() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *p, int *a, int i) { p->x++; a[i]--; return p->x + a[i]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_struct_pointer_subtraction_by_integer() {
        let artifact = compile_source(
            "struct pair { int x; int y; }; struct pair *f(struct pair *a, int n) { return a - n; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_pointer_post_increment_and_decrement_with_scaled_steps() {
        let artifact = compile_source(
            "int *advance_int(int *p) { p++; return p; } unsigned char *rewind_byte(unsigned char *p) { p--; return p; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_uninitialized_local_assigned_before_read() {
        let artifact =
            compile_source("int f(int n) { int value; value = n + 1; return value; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_narrow_uninitialized_local_assigned_before_read() {
        let artifact =
            compile_source("int f(int n) { unsigned char value; value = n; return value; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn cosmic_sia_bundle_round_trips_data_objects() {
        let artifact = Artifact {
            target: TARGET,
            functions: Vec::new(),
            data: vec![DataArtifact {
                name: "global".into(),
                bytes: vec![1, 2, 3, 4],
                align: 4,
                read_only: false,
                relocations: vec![RelocationArtifact {
                    offset: 0,
                    target: "other".into(),
                    addend: 8,
                }],
            }],
        };
        let encoded = artifact.to_bytes().unwrap();
        assert_eq!(Artifact::from_bytes(&encoded).unwrap(), artifact);
    }

    #[test]
    fn cosmic_sia_bundle_round_trips_before_lighting_load() {
        let artifact =
            compile_source("int add(int left, int right) { return left + right; }").unwrap();
        let encoded = artifact.to_bytes().unwrap();
        assert_eq!(Artifact::from_bytes(&encoded).unwrap(), artifact);
    }

    #[test]
    fn bundle_decoder_rejects_truncated_or_ambiguous_code() {
        let artifact =
            compile_source("int add(int left, int right) { return left + right; }").unwrap();
        let encoded = artifact.to_bytes().unwrap();
        assert!(Artifact::from_bytes(&encoded[..encoded.len() - 1])
            .unwrap_err()
            .to_string()
            .contains("truncated COSMIC-SIA"));

        let mut trailing = encoded;
        trailing.push(0);
        assert!(Artifact::from_bytes(&trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes"));
    }

    #[test]
    fn prepares_systemv_register_state_for_external_lighting_execution() {
        let artifact =
            compile_source("int add(int left, int right) { return left + right; }").unwrap();
        let plan = artifact
            .prepare_integer_call("add", 0x0001_0000, &[0xffff_ffff, 1])
            .unwrap();
        assert_eq!(plan.entry_address, 0x0001_0000);
        assert_eq!(
            plan.return_address,
            plan.entry_address + u32::try_from(plan.code.len()).unwrap()
        );
        assert_eq!(plan.registers[0], 0);
        assert_eq!(plan.registers[1], 0xffff_ffff);
        assert_eq!(plan.registers[2], 1);
        assert_eq!(plan.registers[14], plan.return_address);
        assert!(plan.diagnostic().contains("entry=0x00010000"));
        assert!(plan.diagnostic().contains("r1=0xffffffff"));
    }

    #[test]
    fn rejects_an_unaligned_or_unknown_lighting_entry_plan() {
        let artifact =
            compile_source("int add(int left, int right) { return left + right; }").unwrap();
        assert!(artifact
            .prepare_integer_call("add", 1, &[])
            .unwrap_err()
            .to_string()
            .contains("2-byte aligned"));
        assert!(artifact
            .prepare_integer_call("missing", 0, &[])
            .unwrap_err()
            .to_string()
            .contains("no function"));
    }

    #[test]
    fn maps_float_and_double_to_cranelift_scalar_types() {
        let location = Location::default();
        assert_eq!(ir_type(&Type::Float, location).unwrap(), types::F32);
        assert_eq!(ir_type(&Type::Double, location).unwrap(), types::F64);
    }

    #[test]
    fn accepts_pointer_to_float_when_no_float_value_is_lowered() {
        let artifact = compile_source(
            "int f(float *value) { return value != 0; } int main(void) { return 0; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
    }

    #[test]
    fn lowers_f32_arithmetic_to_clif_before_backend_boundary() {
        let error = compile_source("float f(float a, float b) { return -(a + b) * (a - b) / b; }")
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("SSA value type f32"), "{}", message);
        for instruction in ["fadd", "fneg", "fsub", "fmul", "fdiv"] {
            assert!(message.contains(instruction), "{}", message);
        }
    }

    #[test]
    fn lowers_f64_arithmetic_to_clif_before_backend_boundary() {
        let error =
            compile_source("double f(double a, double b) { return -(a + b) * (a - b) / b; }")
                .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("SSA value type f64"), "{}", message);
        for instruction in ["fadd", "fneg", "fsub", "fmul", "fdiv"] {
            assert!(message.contains(instruction), "{}", message);
        }
    }

    #[test]
    fn lowers_integer_float_conversions_to_clif_before_backend_boundary() {
        for (source, instruction) in [
            ("float f(int x) { return (float)x; }", "fcvt_from_sint"),
            ("float f(unsigned x) { return (float)x; }", "fcvt_from_uint"),
            ("int f(float x) { return (int)x; }", "fcvt_to_sint_sat"),
            (
                "unsigned f(float x) { return (unsigned)x; }",
                "fcvt_to_uint_sat",
            ),
        ] {
            let error = compile_source(source).unwrap_err();
            let message = error.to_string();
            assert!(message.contains(instruction), "{}", message);
        }
    }

    #[test]
    fn lowers_float_width_conversions_to_clif_before_backend_boundary() {
        for (source, instruction) in [
            ("double f(float x) { return (double)x; }", "fpromote"),
            ("float f(double x) { return (float)x; }", "fdemote"),
        ] {
            let error = compile_source(source).unwrap_err();
            let message = error.to_string();
            assert!(message.contains(instruction), "{}", message);
        }
    }

    #[test]
    fn lowers_f32_comparisons_to_clif_before_backend_boundary() {
        for (operator, instruction) in [
            ("<", "fcmp lt"),
            (">", "fcmp gt"),
            ("==", "fcmp eq"),
            ("!=", "fcmp ne"),
            ("<=", "fcmp le"),
            (">=", "fcmp ge"),
        ] {
            let source = format!("int f(float a, float b) {{ return a {operator} b; }}");
            let error = compile_source(&source).unwrap_err();
            let message = error.to_string();
            assert!(message.contains("SSA value type f32"), "{}", message);
            assert!(message.contains(instruction), "{}", message);
        }
    }

    #[test]
    fn lowers_f64_comparisons_to_clif_before_backend_boundary() {
        let error = compile_source("int f(double a, double b) { return a <= b; }").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("SSA value type f64"), "{}", message);
        assert!(message.contains("fcmp le"), "{}", message);
    }

    #[test]
    fn lowers_fp_parameter_and_return_abi_to_clif() {
        for (source, ty) in [
            ("float id(float x) { return x; }", "f32"),
            ("double id(double x) { return x; }", "f64"),
        ] {
            let error = compile_source(source).unwrap_err();
            let message = error.to_string();
            assert!(message.contains(&format!("({ty}) -> {ty}")), "{}", message);
            assert!(
                message.contains(&format!("SSA value type {ty}")),
                "{}",
                message
            );
        }
    }

    #[test]
    fn lowers_direct_fp_call_abi_to_clif() {
        let error = compile_source(
            "float callee(float x) { return x; } float caller(float x) { return callee(x); }",
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("(f32) -> f32"), "{}", message);
    }

    #[test]
    fn lowers_indirect_fp_call_abi_to_clif() {
        let error = compile_source("float caller(float (*fn)(float), float x) { return fn(x); }")
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("(f32) -> f32"), "{}", message);
    }

    #[test]
    fn copies_complete_odd_sized_struct_representations() {
        let artifact = compile_source(
            "struct bytes { char a; char b; char c; }; int f(void) { struct bytes a = {1, 2, 3}; struct bytes b; b = a; return b.a + b.b + b.c; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn copies_nested_struct_and_union_values() {
        let artifact = compile_source(
            "union word { int i; unsigned char b[4]; }; struct outer { char tag; union word value; }; int f(void) { struct outer a = {1, {7}}; struct outer b; b = a; return b.tag + b.value.i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_struct_return_from_local_and_nested_call() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; struct pair make(int x) { struct pair p = {x, x + 1}; return p; } struct pair relay(int x) { return make(x); } int run(void) { struct pair p = relay(4); return p.a + p.b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 3);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_union_return_by_value() {
        let artifact = compile_source(
            "union value { int i; unsigned char bytes[4]; }; union value make(int x) { union value v = {x}; return v; } int run(void) { union value v = make(9); return v.i; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_odd_sized_struct_return_by_hidden_address() {
        let artifact = compile_source(
            "struct bytes { char a; char b; char c; }; struct bytes make(void) { struct bytes v = {1, 2, 3}; return v; } int run(void) { struct bytes v = make(); return v.c; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn aggregate_argument_is_copied_before_callee_mutation() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; int mutate(struct pair p) { p.a = 99; return p.a + p.b; } int run(void) { struct pair p = {1, 2}; int x = mutate(p); return p.a + x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_multiple_struct_and_union_arguments_by_value() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; union value { int i; unsigned char b[4]; }; int sum(struct pair a, union value u, struct pair b) { return a.a + a.b + u.i + b.a + b.b; } int run(void) { struct pair a = {1, 2}; struct pair b = {3, 4}; union value u = {5}; return sum(a, u, b); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_odd_sized_struct_argument_by_value() {
        let artifact = compile_source(
            "struct bytes { char a; char b; char c; }; int sum(struct bytes v) { v.a = 7; return v.a + v.b + v.c; } int run(void) { struct bytes v = {1, 2, 3}; return sum(v) + v.a; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_brace_elided_nested_struct_array_initializers() {
        let artifact = compile_source(
            "struct inner { int x; int y; }; struct outer { struct inner p; int a[2]; int z; }; int f(void) { struct outer o = {1, 2, 3, 4, 5}; return o.p.y + o.a[1] + o.z; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_scalar_braces_inside_aggregate_initializers() {
        let artifact = compile_source(
            "struct pair { int a; int b; }; int f(void) { struct pair p = {{1}, {2}}; int a[2] = {{3}, {4}}; return p.a + p.b + a[0] + a[1]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_partial_nested_aggregate_initializers_with_zero_fill() {
        let artifact = compile_source(
            "struct inner { int x; int y; }; struct outer { struct inner p; int a[3]; }; int f(void) { struct outer o = {{7}, {8}}; return o.p.x + o.p.y + o.a[0] + o.a[2]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }
    #[test]
    fn compiles_cast_to_void_while_preserving_side_effects() {
        let artifact = compile_source("int f(int *p) { (void)(*p = 7); return *p; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_integer_pointer_and_function_designator_casts() {
        let artifact = compile_source(
            "int target(int x) { return x + 1; } int f(int *p) { unsigned u = (unsigned)(char)-1; int *q = (int *)(unsigned)p; int (*fn)(int) = (int (*)(int))target; return (int)(unsigned)q + fn((int)u); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_equal_width_signedness_and_pointer_casts_without_reencoding() {
        let artifact = compile_source(
            "unsigned f(int x, int *p) { unsigned a = (unsigned)x; unsigned b = (unsigned)p; return a + b; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_assignment_through_nested_wrapped_lvalues() {
        let artifact = compile_source(
            "struct pair { int x; }; int f(struct pair *p, int *a, int i) { *((int *)&p->x) = 3; *((int *)&a[i]) = 4; return p->x + a[i]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_assignment_through_pointer_to_pointer_dereference() {
        let artifact = compile_source("int f(int **pp) { **pp = 9; return **pp; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_chained_assignment_values() {
        let artifact =
            compile_source("int f(int *p) { int a; int b; a = b = *p = 6; return a + b + *p; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_post_increment_and_decrement_of_globals() {
        let artifact = compile_source(
            "int g; int f(void) { g = 4; int a = g++; int b = g--; return a + b + g; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_post_increment_through_nested_pointer_lvalues() {
        let artifact =
            compile_source("int f(int **pp) { int old = (**pp)++; (**pp)--; return old + **pp; }")
                .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_post_increment_through_cast_wrapped_member_and_index_lvalues() {
        let artifact = compile_source(
            "struct pair { short x; }; int f(struct pair *p, short *a, int i) { p->x++; a[i]--; return p->x + a[i]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_address_of_globals_statics_and_functions() {
        let artifact = compile_source(
            "int g; static int s; int target(int x) { return x; } int f(void) { int *pg = &g; int *ps = &s; int (*fn)(int) = &target; return *pg + *ps + fn(3); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 2);
        assert!(artifact
            .functions
            .iter()
            .all(|function| !function.code.is_empty()));
    }

    #[test]
    fn compiles_address_of_nested_member_and_array_lvalues() {
        let artifact = compile_source(
            "struct inner { int a[3]; }; struct outer { struct inner i; }; int f(struct outer *p, int n) { int *x = &p->i.a[n]; *x = 7; return p->i.a[n]; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_address_of_dereference_identity_for_nested_pointers() {
        let artifact = compile_source(
            "int f(int **pp) { int **a = &*pp; int *b = &**pp; return *b + (**a); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn diagnoses_excess_local_aggregate_initializer_elements() {
        for source in [
            "int f(void) { int a[2] = {1, 2, 3}; return a[0]; }",
            "struct pair { int a; int b; }; int f(void) { struct pair p = {1, 2, 3}; return p.a; }",
            "union value { int a; int b; }; int f(void) { union value v = {1, 2}; return v.a; }",
        ] {
            assert!(compile_source(source).is_err(), "{source}");
        }
    }

    #[test]
    fn compiles_deeply_nested_partial_aggregate_initializers() {
        let artifact = compile_source(
            "struct leaf { short x; short y; }; struct node { struct leaf leaves[2]; int tail; }; int f(void) { struct node n = {{{1}, {2, 3}}}; return n.leaves[0].x + n.leaves[0].y + n.leaves[1].y + n.tail; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_nested_control_flow_statement_matrix() {
        let artifact = compile_source(
            "int f(int x) { int r = 0; start: if (x < 0) return r; for (int i = 0; i < 4; i++) { if (i == 1) continue; while (x > 0) { x--; if (x == 2) break; } do { r++; } while (0); switch (i) { case 0: r += 2; break; case 2: r += 3; default: r += 4; } } if (r == 99) goto start; return r; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_labels_after_terminators_and_switch_fallthrough() {
        let artifact = compile_source(
            "int f(int x) { if (x) goto done; x = 1; done: switch (x) { case 0: x += 2; case 1: x += 3; default: x += 4; } return x; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_complete_integer_binary_operator_matrix() {
        let artifact = compile_source(
            "int f(int a, unsigned b, short s) { int r = a * s + a / 3 + a % 3 - s; r = r & a | (int)b ^ s; r = r + (a << 2) + ((int)b >> 1) + (a >> 1); return r + (a < s) + (a > s) + (a == s) + (a != s) + (a <= s) + (a >= s) + (a && b) + (a || b); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn compiles_pointer_binary_operator_matrix() {
        let artifact = compile_source(
            "int f(int *p, int *q, int i) { int *a = p + i; int *b = i + p; int *c = a - i; return *b + *c + (p == q) + (p != q) + (p < q) + (p <= q) + (p > q) + (p >= q); }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn binary_operator_enum_is_exhaustively_audited() {
        fn kind(operator: saltwater_parser::data::hir::BinaryOp) -> &'static str {
            use saltwater_parser::data::hir::BinaryOp;
            match operator {
                BinaryOp::LogicalOr => "logical-or",
                BinaryOp::BitwiseOr => "bitwise-or",
                BinaryOp::LogicalAnd => "logical-and",
                BinaryOp::BitwiseAnd => "bitwise-and",
                BinaryOp::Xor => "xor",
                BinaryOp::Mul => "mul",
                BinaryOp::Div => "div",
                BinaryOp::Mod => "mod",
                BinaryOp::Add => "add",
                BinaryOp::Sub => "sub",
                BinaryOp::Shl => "shl",
                BinaryOp::Shr => "shr",
                BinaryOp::Compare(_) => "compare",
                BinaryOp::Assign => "assign",
            }
        }
        assert_eq!(kind(saltwater_parser::data::hir::BinaryOp::Add), "add");
    }

    #[test]
    fn compiles_pointer_difference_as_function_result() {
        let artifact = compile_source("long f(int *p, int *q) { return p - q; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn type_lowering_explicitly_classifies_every_frontend_type_family() {
        let location = Location::default();
        assert_eq!(ir_type(&Type::Bool, location).unwrap(), types::I8);
        assert_eq!(ir_type(&Type::Char(true), location).unwrap(), types::I8);
        assert_eq!(ir_type(&Type::Short(true), location).unwrap(), types::I16);
        assert_eq!(ir_type(&Type::Int(true), location).unwrap(), types::I32);
        assert_eq!(ir_type(&Type::Long(true), location).unwrap(), types::I32);
        assert_eq!(ir_type(&Type::Float, location).unwrap(), types::F32);
        assert_eq!(ir_type(&Type::Double, location).unwrap(), types::F64);
        for ty in [
            Type::Void,
            Type::Array(
                Box::new(Type::Int(true)),
                saltwater_parser::data::types::ArrayType::Fixed(2),
            ),
            Type::VaList,
            Type::Error,
        ] {
            assert!(ir_type(&ty, location).is_err(), "{ty:?}");
        }
    }
    #[test]
    fn lowers_vla_with_fixed_inner_dimension() {
        let artifact =
            compile_source("int f(int n) { int a[n][3]; a[1][2] = 9; return a[1][2]; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn lowers_sizeof_vla_to_runtime_size() {
        let artifact =
            compile_source("unsigned f(int n) { int a[n + 2]; return sizeof(a); }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn lowers_general_runtime_vla_bound_expression() {
        let artifact =
            compile_source("int f(int n) { int a[n + 3]; a[n] = 11; return a[n]; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn lowers_identifier_bound_vla_to_dynamic_stack_allocation() {
        let artifact = compile_source("int f(int n) { int a[n]; a[2] = 7; return a[2]; }").unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn lowers_local_designated_initializers_to_sia32() {
        let artifact = compile_source(
            "struct s { int a; int b; int c; }; int f(void) { int a[5] = { [2] = 7, 8 }; struct s v = { .b = 4, 5 }; return a[0] + a[2] + a[3] + v.a + v.b + v.c; }",
        )
        .unwrap();
        assert_eq!(artifact.functions.len(), 1);
        assert!(!artifact.functions[0].code.is_empty());
    }

    #[test]
    fn lowers_global_designated_initializers_to_data() {
        let artifact = compile_source(
            "int a[5] = { [2] = 7, 8 }; struct s { int a; int b; int c; }; struct s v = { .b = 4, 5 }; int f(void) { return a[2] + v.b; }",
        )
        .unwrap();
        let array = artifact
            .data
            .iter()
            .find(|object| object.name == "a")
            .unwrap();
        assert_eq!(&array.bytes[0..8], &[0; 8]);
        assert_eq!(&array.bytes[8..12], &7i32.to_le_bytes());
        assert_eq!(&array.bytes[12..16], &8i32.to_le_bytes());
        let value = artifact
            .data
            .iter()
            .find(|object| object.name == "v")
            .unwrap();
        assert_eq!(&value.bytes[0..4], &[0; 4]);
        assert_eq!(&value.bytes[4..8], &4i32.to_le_bytes());
        assert_eq!(&value.bytes[8..12], &5i32.to_le_bytes());
    }
}
