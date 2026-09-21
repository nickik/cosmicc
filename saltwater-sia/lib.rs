//! Cosmic C's initial SIA32 lowering path.
//!
//! This crate deliberately emits only SIA32 code.  It does not fall back to a
//! host ISA, and it rejects floating-point C before creating Cranelift IR.

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::fmt;

use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, AbiParam, Block, ExtFuncData, ExternalName, Function, InstBuilder, MemFlagsData,
    Signature, StackSlot, StackSlotData, StackSlotKind, UserExternalName, UserFuncName, Value,
};
use cranelift_codegen::isa::{self, CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::Context;
use cranelift_codegen::RelocTarget;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use saltwater_parser::check_semantics;
pub use saltwater_parser::data::error::LexError;
use saltwater_parser::data::types::{FunctionType, StructType};
use saltwater_parser::data::{
    hir::{Declaration, Expr, ExprType, Initializer, LiteralValue, Stmt, StmtType, Symbol},
    CompileError, Location, StorageClass, Type,
};
pub use saltwater_parser::{preprocess, Opt};
use target_lexicon::Triple;

/// The fixed target accepted by this compiler stage.
pub const TARGET: &str = "sia32-unknown-none";

const BUNDLE_MAGIC: &[u8] = b"COSMIC-SIA\0";
const BUNDLE_VERSION: u16 = 2;
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
        if cursor != bytes.len() {
            return Err(Error::Codegen(
                "COSMIC-SIA bundle has trailing bytes".into(),
            ));
        }
        let artifact = Self {
            target: TARGET,
            functions,
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
        if self.functions.is_empty() {
            return Err(Error::Codegen(
                "COSMIC-SIA bundle contains no functions".into(),
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
                    "COSMIC-SIA bundle contains duplicate function `{}`",
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

/// Compile C source to SIA32 instructions through the production Cranelift backend.
pub fn compile(source: &str, opt: Opt) -> Result<Artifact, Error> {
    let program = check_semantics(source, opt);
    let declarations = program.result.map_err(Error::Source)?;

    for declaration in &declarations {
        if declaration_uses_float(&declaration.data) {
            return Err(unsupported(
                declaration.location,
                "floating-point C is not supported for the SIA32 target",
            ));
        }
    }

    let isa = target_isa()?;
    let function_indices: HashMap<Symbol, u32> = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            matches!(declaration.data.symbol.get().ctype, Type::Function(_))
                .then_some((declaration.data.symbol, index as u32))
        })
        .collect();
    let mut functions = Vec::new();
    for (index, declaration) in declarations.iter().enumerate() {
        let metadata = declaration.data.symbol.get();
        if metadata.storage_class == StorageClass::Typedef {
            continue;
        }
        let function_type = match &metadata.ctype {
            Type::Function(function_type) => function_type,
            _ if metadata.storage_class == StorageClass::Extern
                && declaration.data.init.is_none() =>
            {
                // A declaration-only extern object allocates no storage in this
                // translation unit. Accept it here; an actual reference still
                // requires global-symbol lowering.
                continue;
            }
            _ => {
                return Err(unsupported(
                    declaration.location,
                    format!(
                        "SIA32 top-level data unsupported: symbol={:?}, metadata={metadata:?}, init={:?}",
                        declaration.data.symbol,
                        declaration.data.init,
                    ),
                ));
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
    isa: &dyn TargetIsa,
) -> Result<FunctionArtifact, Error> {
    let mut signature = Signature::new(CallConv::SystemV);
    let parameters = function_parameters(function_type);
    for parameter in parameters {
        signature
            .params
            .push(AbiParam::new(ir_type(&parameter.get().ctype, location)?));
    }
    if !matches!(*function_type.return_type, Type::Void) {
        signature.returns.push(AbiParam::new(ir_type(
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
            let mut lowerer = FunctionLowerer::new(&mut builder, function_indices);
            for (parameter, value) in parameters.iter().zip(entry_values.iter()) {
                let variable = lowerer
                    .builder
                    .declare_var(ir_type(&parameter.get().ctype, location)?);
                lowerer.builder.def_var(variable, *value);
                lowerer.variables.insert(*parameter, variable);
                lowerer
                    .variable_types
                    .insert(*parameter, ir_type(&parameter.get().ctype, location)?);
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
                function_indices
                    .iter()
                    .find_map(|(symbol, index)| {
                        (*index == user.index).then(|| symbol.get().id.resolve_and_clone())
                    })
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "SIA32 emitted unknown function relocation in {name}"
                        ))
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

fn function_parameters(function_type: &FunctionType) -> &[Symbol] {
    if function_type.params.len() == 1 && function_type.params[0].get().ctype == Type::Void {
        &[]
    } else {
        &function_type.params
    }
}

struct FunctionLowerer<'a, 'b, 'c> {
    builder: &'a mut FunctionBuilder<'b>,
    variables: HashMap<Symbol, Variable>,
    variable_types: HashMap<Symbol, cranelift_codegen::ir::Type>,
    stack_locals: HashMap<Symbol, StackSlot>,
    function_indices: &'c HashMap<Symbol, u32>,
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
            function_indices,
            return_type,
            loop_targets: Vec::new(),
            break_targets: Vec::new(),
            switch_cases: Vec::new(),
            labels: HashMap::new(),
            terminated: false,
        }
    }

    fn compile_stmt(&mut self, statement: &Stmt) -> Result<(), Error> {
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
                if let Some(value) = value {
                    let mut value = self.compile_expr(value)?;
                    if let Some(return_type) = self.return_type {
                        let value_type = self.builder.func.dfg.value_type(value);
                        if value_type != return_type {
                            value = self.builder.ins().uextend(return_type, value);
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
                let condition = self.compile_expr(condition)?;
                let condition_ty = self.builder.func.dfg.value_type(condition);
                let zero = self.builder.ins().iconst(condition_ty, 0);
                let condition = self.builder.ins().icmp(IntCC::NotEqual, condition, zero);

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
                let condition = self.compile_expr(condition)?;
                let condition_ty = self.builder.func.dfg.value_type(condition);
                let zero = self.builder.ins().iconst(condition_ty, 0);
                let condition = self.builder.ins().icmp(IntCC::NotEqual, condition, zero);
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
                    let condition = self.compile_expr(condition)?;
                    let condition_ty = self.builder.func.dfg.value_type(condition);
                    let zero = self.builder.ins().iconst(condition_ty, 0);
                    let condition = self.builder.ins().icmp(IntCC::NotEqual, condition, zero);
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
                let condition = self.compile_expr(condition)?;
                let condition_ty = self.builder.func.dfg.value_type(condition);
                let zero = self.builder.ins().iconst(condition_ty, 0);
                let condition = self.builder.ins().icmp(IntCC::NotEqual, condition, zero);
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
            _ => Err(unsupported(
                statement.location,
                format!("SIA32 control-flow lowering is not implemented for {statement:?}"),
            )),
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
        if matches!(
            metadata.ctype,
            Type::Struct(_) | Type::Union(_) | Type::Array(_, _)
        ) {
            let size = u32::try_from(
                metadata
                    .ctype
                    .sizeof()
                    .map_err(|_| unsupported(location, "aggregate local has incomplete type"))?,
            )
            .map_err(|_| unsupported(location, "aggregate local is too large for SIA32"))?;
            let align = metadata
                .ctype
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
                self.initialize_stack_aggregate(slot, &metadata.ctype, initializer, location)?;
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

    fn initialize_stack_aggregate(
        &mut self,
        slot: StackSlot,
        ctype: &Type,
        initializer: &Initializer,
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
                let element_size = element
                    .sizeof()
                    .map_err(|_| unsupported(location, "array element has incomplete type"))?;
                for (index, item) in items.iter().enumerate() {
                    if u64::try_from(index).unwrap_or(u64::MAX) >= *count {
                        break;
                    }
                    let Initializer::Scalar(expression) = item else {
                        return Err(unsupported(
                            location,
                            "nested aggregate initialization is not supported for SIA32 yet",
                        ));
                    };
                    let value = self.compile_expr(expression)?;
                    let value_ty = ir_type(element, location)?;
                    let value = self.coerce_integer_value(value, value_ty, &expression.ctype);
                    let offset = i32::try_from((index as u64) * element_size).map_err(|_| {
                        unsupported(location, "aggregate initializer offset is too large")
                    })?;
                    self.builder
                        .ins()
                        .stack_store(types::I32, value, slot, offset);
                }
                Ok(())
            }
            Type::Struct(struct_type) => {
                let mut offset = 0u64;
                for (field, item) in struct_type.members().iter().zip(items.iter()) {
                    let align = field.ctype.alignof().map_err(|_| {
                        unsupported(location, "struct field has unsupported alignment")
                    })?;
                    let rem = offset % align;
                    if rem != 0 {
                        offset += align - rem;
                    }
                    let Initializer::Scalar(expression) = item else {
                        return Err(unsupported(
                            location,
                            "nested aggregate initialization is not supported for SIA32 yet",
                        ));
                    };
                    let value = self.compile_expr(expression)?;
                    let value_ty = ir_type(&field.ctype, location)?;
                    let value = self.coerce_integer_value(value, value_ty, &expression.ctype);
                    let field_offset = i32::try_from(offset).map_err(|_| {
                        unsupported(location, "aggregate initializer offset is too large")
                    })?;
                    self.builder
                        .ins()
                        .stack_store(types::I32, value, slot, field_offset);
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
            ExprType::Deref(pointer) => self.compile_expr(pointer),
            ExprType::Member(base, member) => {
                self.compile_member_address(base, *member, lvalue.location)
            }
            // Saltwater lowers subscripting into pointer arithmetic and can
            // leave that Binary(Add, ...) directly as the lvalue.
            ExprType::Binary(saltwater_parser::data::hir::BinaryOp::Add, _, _) => {
                self.compile_expr(lvalue)
            }
            ExprType::Noop(inner) => self.compile_lvalue_address(inner),
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
                } else {
                    let variable = self.variables.get(symbol).copied().ok_or_else(|| {
                        unsupported(
                            location,
                            "direct aggregate member base has no SIA32 storage",
                        )
                    })?;
                    self.builder.use_var(variable)
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
        let ty = if matches!(expression.ctype, Type::Void) {
            types::I32
        } else {
            ir_type(&expression.ctype, expression.location)?
        };
        match &expression.expr {
            ExprType::Id(symbol) => {
                if let Some(variable) = self.variables.get(symbol).copied() {
                    Ok(self.builder.use_var(variable))
                } else if let Some(slot) = self.stack_locals.get(symbol).copied() {
                    Ok(self.builder.ins().stack_addr(types::I32, slot, 0))
                } else {
                    Err(unsupported(
                        expression.location,
                        "taking addresses of globals or unsupported objects is not supported for SIA32 yet",
                    ))
                }
            }
            ExprType::StaticRef(value) => {
                let mut lvalue = value.as_ref();
                while let ExprType::Noop(inner) = &lvalue.expr {
                    lvalue = inner;
                }
                match &lvalue.expr {
                    // &*p is exactly p and does not perform a load.
                    ExprType::Deref(pointer) => self.compile_expr(pointer),
                    // Member-address lowering already computes the lvalue address.
                    ExprType::Member(base, member) => {
                        self.compile_member_address(base, *member, expression.location)
                    }
                    // Address-taken scalar locals require stack-slot lowering; do not
                    // silently manufacture an address for an SSA variable.
                    ExprType::Id(_) => Err(unsupported(
                        expression.location,
                        "taking the address of an SSA local requires SIA32 stack-local support",
                    )),
                    _ => Err(unsupported(
                        expression.location,
                        format!("SIA32 address-of lowering is not implemented for {lvalue:?}"),
                    )),
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
            ExprType::Literal(LiteralValue::Float(_)) => Err(unsupported(
                expression.location,
                "floating-point C is not supported for the SIA32 target",
            )),
            ExprType::Literal(LiteralValue::Str(_)) => Err(unsupported(
                expression.location,
                "string literals require SIA32 global-data support",
            )),
            ExprType::Noop(value) => self.compile_expr(value),
            ExprType::Cast(value) => {
                let value_clif = self.compile_expr(value)?;
                let source_ty = self.builder.func.dfg.value_type(value_clif);
                let dest_ty = ty;

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
                let bytes = sized.sizeof().map_err(|_| {
                    unsupported(expression.location, "sizeof requires a complete SIA32 type")
                })?;
                Ok(self.builder.ins().iconst(ty, bytes as i64))
            }
            ExprType::Comma(left, right) => {
                let _ = self.compile_expr(left)?;
                self.compile_expr(right)
            }
            ExprType::Member(base, member) => {
                let address = self.compile_member_address(base, *member, expression.location)?;
                Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
            }
            // The established HIR represents an ordinary C local read as
            // `Deref(Id(symbol))`: `Id` creates the lvalue address and Deref
            // loads it. This backend keeps non-address-taken locals in SSA,
            // so this pair becomes a direct `use_var` instead of a memory load.
            ExprType::Deref(pointer) => match &pointer.expr {
                ExprType::Id(symbol) => {
                    let variable = self.variables.get(symbol).copied().ok_or_else(|| {
                        unsupported(
                            expression.location,
                            format!(
                                "SIA32 Deref(Id) has no local mapping: symbol={symbol:?}, expr={expression:?}"
                            ),
                        )
                    })?;
                    Ok(self.builder.use_var(variable))
                }
                ExprType::Member(base, member) => {
                    let address =
                        self.compile_member_address(base, *member, expression.location)?;
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
                ExprType::Noop(inner) => {
                    if let ExprType::Member(base, member) = &inner.expr {
                        let address =
                            self.compile_member_address(base, *member, expression.location)?;
                        Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                    } else {
                        let address = self.compile_expr(pointer)?;
                        Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                    }
                }
                _ => {
                    let address = self.compile_expr(pointer)?;
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
            },
            ExprType::Negate(value) => {
                let value = self.compile_expr(value)?;
                let value_ty = self.builder.func.dfg.value_type(value);
                let zero = self.builder.ins().iconst(value_ty, 0);
                Ok(self.builder.ins().isub(zero, value))
            }
            ExprType::BitwiseNot(value) => {
                let value = self.compile_expr(value)?;
                let all_ones = self.builder.ins().iconst(ty, -1);
                Ok(self.builder.ins().bxor(value, all_ones))
            }
            ExprType::PostIncrement(value, increment) => {
                let mut lvalue = value.as_ref();
                while let ExprType::Noop(inner) = &lvalue.expr {
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

                match &lvalue.expr {
                    ExprType::Id(symbol) => {
                        let variable = self.variables.get(symbol).copied().ok_or_else(|| {
                            unsupported(
                                expression.location,
                                "SIA32 post-increment/decrement target is not a mapped local",
                            )
                        })?;
                        self.builder.def_var(variable, updated);
                    }
                    ExprType::Member(base, member) => {
                        let address =
                            self.compile_member_address(base, *member, expression.location)?;
                        self.builder
                            .ins()
                            .store(MemFlagsData::new(), updated, address, 0);
                    }
                    ExprType::Deref(pointer) => {
                        let address = self.compile_expr(pointer)?;
                        self.builder
                            .ins()
                            .store(MemFlagsData::new(), updated, address, 0);
                    }
                    _ => {
                        return Err(unsupported(
                            expression.location,
                            format!(
                                "SIA32 post-increment/decrement lowering is not implemented for {lvalue:?}"
                            ),
                        ));
                    }
                }

                Ok(old)
            }
            ExprType::Ternary(condition, yes, no) => {
                let condition = self.compile_expr(condition)?;
                let condition_ty = self.builder.func.dfg.value_type(condition);
                let zero = self.builder.ins().iconst(condition_ty, 0);
                let condition = self.builder.ins().icmp(IntCC::NotEqual, condition, zero);

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
                let yes_value = self.coerce_integer_value(yes_value, ty, &yes.ctype);
                self.builder.ins().jump(merge_block, &[yes_value.into()]);

                self.builder.switch_to_block(no_block);
                self.builder.seal_block(no_block);
                let no_value = self.compile_expr(no)?;
                let no_value = self.coerce_integer_value(no_value, ty, &no.ctype);
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
                    while let ExprType::Noop(inner) = &assignment_left.expr {
                        assignment_left = inner;
                    }

                    let value = self.compile_expr(right)?;

                    // The analyzer may represent an ordinary local assignment
                    // either directly as Id(symbol) or as Deref(Id(symbol)).
                    // Both denote the same SSA local in this backend.
                    if let ExprType::Id(symbol) = &assignment_left.expr {
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
                        return Err(unsupported(
                            left.location,
                            format!(
                                "SIA32 assignment to unmapped Id: symbol={symbol:?}, lhs={left:?}"
                            ),
                        ));
                    }

                    if let ExprType::Member(base, member) = &assignment_left.expr {
                        let address = self.compile_member_address(base, *member, left.location)?;
                        let target_ty = ir_type(&assignment_left.ctype, left.location)?;
                        let value = self.coerce_integer_value(value, target_ty, &right.ctype);
                        self.builder
                            .ins()
                            .store(MemFlagsData::new(), value, address, 0);
                        return Ok(value);
                    }

                    if matches!(
                        assignment_left.expr,
                        ExprType::Binary(saltwater_parser::data::hir::BinaryOp::Add, _, _)
                    ) {
                        let address = self.compile_lvalue_address(assignment_left)?;
                        let target_ty = ir_type(&assignment_left.ctype, left.location)?;
                        let value = self.coerce_integer_value(value, target_ty, &right.ctype);
                        self.builder
                            .ins()
                            .store(MemFlagsData::new(), value, address, 0);
                        return Ok(value);
                    }

                    let ExprType::Deref(pointer) = &assignment_left.expr else {
                        return Err(unsupported(
                            left.location,
                            format!(
                                "SIA32 assignment lowering is not implemented for lhs {left:?}"
                            ),
                        ));
                    };

                    if let ExprType::Id(symbol) = &pointer.expr {
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
                    }

                    let address = self.compile_expr(pointer)?;
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
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;

                // The analyzer records C's usual arithmetic-conversion result
                // on the binary expression. Normalize both operands to that
                // width before emitting CLIF. This avoids type-mismatch IR for
                // combinations such as char + int and short < long.
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
                    _ => {
                        return Err(unsupported(
                            expression.location,
                            format!("SIA32 operator lowering is not implemented for {operator:?}"),
                        ));
                    }
                };
                Ok(value)
            }
            ExprType::FuncCall(function, arguments) => {
                let ExprType::Id(symbol) = &function.expr else {
                    return Err(unsupported(
                        expression.location,
                        "SIA32 indirect calls are not supported yet",
                    ));
                };
                let metadata = symbol.get();
                let Type::Function(function_type) = &metadata.ctype else {
                    return Err(unsupported(
                        expression.location,
                        "SIA32 call target is not a function",
                    ));
                };
                let function_index =
                    self.function_indices.get(symbol).copied().ok_or_else(|| {
                        unsupported(
                            expression.location,
                            format!(
                                "SIA32 direct call target `{}` has no translation-unit definition",
                                metadata.id.resolve_and_clone()
                            ),
                        )
                    })?;
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
                for parameter in parameters {
                    signature.params.push(AbiParam::new(ir_type(
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
                if !matches!(*function_type.return_type, Type::Void) {
                    signature.returns.push(AbiParam::new(ir_type(
                        &function_type.return_type,
                        expression.location,
                    )?));
                }
                let external = self
                    .builder
                    .func
                    .declare_imported_user_function(UserExternalName::new(0, function_index));
                let signature = self.builder.import_signature(signature);
                let function_ref = self.builder.import_function(ExtFuncData {
                    name: ExternalName::user(external),
                    signature,
                    colocated: true,
                    patchable: false,
                });
                let mut values = Vec::with_capacity(arguments.len());
                for (index, argument) in arguments.iter().enumerate() {
                    let value = self.compile_expr(argument)?;
                    let parameter_ty = if let Some(parameter) = parameters.get(index) {
                        ir_type(&parameter.get().ctype, expression.location)?
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
                let call = self.builder.ins().call(function_ref, &values);
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
    let ty = match ctype {
        Type::Bool | Type::Char(_) => types::I8,
        Type::Short(_) => types::I16,
        Type::Int(_) | Type::Long(_) | Type::Enum(_, _) | Type::Pointer(_, _) => types::I32,
        Type::Float | Type::Double => {
            return Err(unsupported(
                location,
                "floating-point C is not supported for the SIA32 target",
            ));
        }
        _ => {
            return Err(unsupported(
                location,
                format!("SIA32 type lowering is not implemented for {ctype:?}"),
            ))
        }
    };
    Ok(ty)
}

fn declaration_uses_float(declaration: &Declaration) -> bool {
    type_uses_float(&declaration.symbol.get().ctype, &mut HashSet::new())
        || declaration
            .init
            .as_ref()
            .map_or(false, initializer_uses_float)
}

fn initializer_uses_float(initializer: &Initializer) -> bool {
    match initializer {
        Initializer::Scalar(expression) => expr_uses_float(expression),
        Initializer::InitializerList(items) => items.iter().any(initializer_uses_float),
        Initializer::FunctionBody(statements) => statements.iter().any(stmt_uses_float),
    }
}

fn stmt_uses_float(statement: &Stmt) -> bool {
    match &statement.data {
        StmtType::Compound(statements) => statements.iter().any(stmt_uses_float),
        StmtType::If(condition, yes, no) => {
            expr_uses_float(condition)
                || stmt_uses_float(yes)
                || no.as_deref().map_or(false, stmt_uses_float)
        }
        StmtType::Do(body, condition) | StmtType::While(condition, body) => {
            expr_uses_float(condition) || stmt_uses_float(body)
        }
        StmtType::For(init, condition, step, body) => {
            stmt_uses_float(init)
                || condition.as_deref().map_or(false, expr_uses_float)
                || step.as_deref().map_or(false, expr_uses_float)
                || stmt_uses_float(body)
        }
        StmtType::Switch(expression, body) => expr_uses_float(expression) || stmt_uses_float(body),
        StmtType::Label(_, body) | StmtType::Case(_, body) | StmtType::Default(body) => {
            stmt_uses_float(body)
        }
        StmtType::Expr(expression) => expr_uses_float(expression),
        StmtType::Return(value) => value.as_ref().map_or(false, expr_uses_float),
        StmtType::Decl(declarations) => declarations
            .iter()
            .any(|decl| declaration_uses_float(&decl.data)),
        StmtType::Goto(_) | StmtType::Continue | StmtType::Break => false,
    }
}

fn expr_uses_float(expression: &Expr) -> bool {
    type_uses_float(&expression.ctype, &mut HashSet::new())
        || match &expression.expr {
            ExprType::Literal(LiteralValue::Float(_)) => true,
            ExprType::FuncCall(function, arguments) => {
                expr_uses_float(function) || arguments.iter().any(expr_uses_float)
            }
            ExprType::Member(value, _)
            | ExprType::PostIncrement(value, _)
            | ExprType::Cast(value)
            | ExprType::Deref(value)
            | ExprType::Negate(value)
            | ExprType::BitwiseNot(value)
            | ExprType::StaticRef(value)
            | ExprType::Noop(value) => expr_uses_float(value),
            ExprType::Sizeof(ctype) => type_uses_float(ctype, &mut HashSet::new()),
            ExprType::Binary(_, left, right) | ExprType::Comma(left, right) => {
                expr_uses_float(left) || expr_uses_float(right)
            }
            ExprType::Ternary(condition, yes, no) => {
                expr_uses_float(condition) || expr_uses_float(yes) || expr_uses_float(no)
            }
            ExprType::Id(_) | ExprType::Literal(_) => false,
        }
}

fn type_uses_float(ctype: &Type, seen_structs: &mut HashSet<String>) -> bool {
    match ctype {
        Type::Float | Type::Double => true,
        Type::Pointer(pointee, _) => type_uses_float(pointee, seen_structs),
        Type::Array(element, _) => type_uses_float(element, seen_structs),
        Type::Function(function) => {
            type_uses_float(&function.return_type, seen_structs)
                || function
                    .params
                    .iter()
                    .any(|parameter| type_uses_float(&parameter.get().ctype, seen_structs))
        }
        Type::Struct(struct_type) | Type::Union(struct_type) => {
            struct_uses_float(struct_type, seen_structs)
        }
        _ => false,
    }
}

fn struct_uses_float(struct_type: &StructType, seen_structs: &mut HashSet<String>) -> bool {
    let name = match struct_type {
        StructType::Named(name, _) => Some(name.resolve_and_clone()),
        StructType::Anonymous(_) => None,
    };
    if let Some(name) = &name {
        if !seen_structs.insert(name.clone()) {
            return false;
        }
    }
    struct_type
        .members()
        .iter()
        .any(|member| type_uses_float(&member.ctype, seen_structs))
}

fn unsupported(location: Location, message: impl Into<String>) -> Error {
    Error::Unsupported {
        location,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_source(source: &str) -> Result<Artifact, Error> {
        compile(source, Opt::default())
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
    fn compiles_scalar_post_increment_and_decrement() {
        let artifact = compile_source(
            "int update(int n) { int i = 0; int old = i++; int prior = i--; return old + prior + i + n; }",
        )
        .unwrap();
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
            "unexpected diagnostic: {}", message
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
    fn compiles_pointer_arithmetic_lvalue_assignment() {
        let artifact =
            compile_source("int store(int *p, int i, int x) { p[i] = x; return p[i]; }").unwrap();
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
    fn compiles_wrapped_and_nested_aggregate_member_addresses() {
        let artifact = compile_source(
            "struct inner { int x; }; struct outer { struct inner i; }; int f(struct outer *p) { p->i.x = 7; return p->i.x; }",
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
    fn rejects_float_declarations_before_backend_lowering() {
        let error = compile_source("int main(void) { float x = 1.0; return 0; }").unwrap_err();
        assert!(error
            .to_string()
            .contains("floating-point C is not supported"));
    }

    #[test]
    fn rejects_float_literals_even_when_cast_to_int() {
        let error = compile_source("int main(void) { return (int)1.0; }").unwrap_err();
        assert!(error
            .to_string()
            .contains("floating-point C is not supported"));
    }

    #[test]
    fn rejects_pointers_to_float_too() {
        let error =
            compile_source("int f(float *value) { return 0; } int main(void) { return 0; }")
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("floating-point C is not supported"));
    }
}
