//! Cosmic C's initial SIA32 lowering path.
//!
//! This crate deliberately emits only SIA32 code.  It does not fall back to a
//! host ISA, and it rejects floating-point C before creating Cranelift IR.

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::fmt;

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, AbiParam, Function, InstBuilder, MemFlagsData, Signature, UserFuncName, Value,
};
use cranelift_codegen::isa::{self, CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::Context;
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
const BUNDLE_VERSION: u16 = 1;
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
            functions.push(FunctionArtifact { name, code });
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
    let mut functions = Vec::new();
    for (index, declaration) in declarations.iter().enumerate() {
        let metadata = declaration.data.symbol.get();
        if metadata.storage_class == StorageClass::Typedef {
            continue;
        }
        let function_type = match &metadata.ctype {
            Type::Function(function_type) => function_type,
            _ => {
                return Err(unsupported(
                    declaration.location,
                    "global data is not implemented in the initial SIA32 compiler path",
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
    isa: &dyn TargetIsa,
) -> Result<FunctionArtifact, Error> {
    if function_type.varargs {
        return Err(unsupported(
            location,
            "variadic functions are not supported for SIA32 yet",
        ));
    }
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
            let mut lowerer = FunctionLowerer::new(&mut builder);
            for (parameter, value) in parameters.iter().zip(entry_values.iter()) {
                let variable = lowerer
                    .builder
                    .declare_var(ir_type(&parameter.get().ctype, location)?);
                lowerer.builder.def_var(variable, *value);
                lowerer.variables.insert(*parameter, variable);
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
        builder.finalize(isa.frontend_config());
    }

    let mut control_plane = ControlPlane::default();
    let compiled = context.compile(isa, &mut control_plane).map_err(|error| {
        Error::Codegen(format!("SIA32 lowering failed for {name}: {}", error.inner))
    })?;
    let code = compiled.code_buffer().to_vec();
    if code.is_empty() || code.len() % 2 != 0 {
        return Err(Error::Codegen(format!(
            "SIA32 emitted invalid instruction bytes for {name}"
        )));
    }
    Ok(FunctionArtifact { name, code })
}

fn function_parameters(function_type: &FunctionType) -> &[Symbol] {
    if function_type.params.len() == 1 && function_type.params[0].get().ctype == Type::Void {
        &[]
    } else {
        &function_type.params
    }
}

struct FunctionLowerer<'a, 'b> {
    builder: &'a mut FunctionBuilder<'b>,
    variables: HashMap<Symbol, Variable>,
    terminated: bool,
}

impl<'a, 'b> FunctionLowerer<'a, 'b> {
    fn new(builder: &'a mut FunctionBuilder<'b>) -> Self {
        Self {
            builder,
            variables: HashMap::new(),
            terminated: false,
        }
    }

    fn compile_stmt(&mut self, statement: &Stmt) -> Result<(), Error> {
        if self.terminated {
            return Ok(());
        }
        match &statement.data {
            StmtType::Compound(statements) => {
                for statement in statements {
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
                    let value = self.compile_expr(value)?;
                    self.builder.ins().return_(&[value]);
                } else {
                    self.builder.ins().return_(&[]);
                }
                self.terminated = true;
                Ok(())
            }
            _ => Err(unsupported(
                statement.location,
                "control flow is not implemented in the initial SIA32 compiler path",
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
        let ty = ir_type(&metadata.ctype, location)?;
        let variable = self.builder.declare_var(ty);
        let initial = match &declaration.init {
            Some(Initializer::Scalar(expression)) => self.compile_expr(expression)?,
            Some(_) => {
                return Err(unsupported(
                    location,
                    "aggregate local initialization is not supported for SIA32 yet",
                ))
            }
            None => {
                return Err(unsupported(
                    location,
                    "uninitialized local variables are not supported for SIA32 yet",
                ))
            }
        };
        self.builder.def_var(variable, initial);
        self.variables.insert(declaration.symbol, variable);
        Ok(())
    }

    fn compile_expr(&mut self, expression: &Expr) -> Result<Value, Error> {
        let ty = ir_type(&expression.ctype, expression.location)?;
        match &expression.expr {
            ExprType::Id(symbol) => {
                let _ = symbol;
                Err(unsupported(
                    expression.location,
                    "taking local addresses and referencing globals are not supported for SIA32 yet",
                ))
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
            ExprType::Cast(value) | ExprType::Noop(value) => self.compile_expr(value),
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
                let struct_type = match &base.ctype {
                    Type::Struct(struct_type) | Type::Union(struct_type) => struct_type,
                    _ => {
                        return Err(unsupported(
                            expression.location,
                            "member access requires a struct or union base",
                        ));
                    }
                };
                let mut offset = 0u64;
                if matches!(&base.ctype, Type::Struct(_)) {
                    let mut found = false;
                    for field in struct_type.members().iter() {
                        let align = field.ctype.alignof().map_err(|_| {
                            unsupported(expression.location, "member has unsupported alignment")
                        })?;
                        let rem = offset % align;
                        if rem != 0 {
                            offset += align - rem;
                        }
                        if field.id == *member {
                            found = true;
                            break;
                        }
                        offset += field.ctype.sizeof().map_err(|_| {
                            unsupported(expression.location, "member has incomplete type")
                        })?;
                    }
                    if !found {
                        return Err(unsupported(expression.location, "unknown struct member"));
                    }
                }
                fn member_base_pointer<'a>(expr: &'a Expr) -> Option<&'a Expr> {
                    match &expr.expr {
                        ExprType::Noop(inner) | ExprType::Cast(inner) => member_base_pointer(inner),
                        ExprType::Deref(pointer) => Some(pointer),
                        _ => None,
                    }
                }
                let pointer = member_base_pointer(base).ok_or_else(|| {
                    unsupported(expression.location, "SIA32 member access requires an addressable aggregate")
                })?;
                let address = match &pointer.expr {
                    ExprType::Id(symbol) => {
                        let variable = self.variables.get(symbol).copied().ok_or_else(|| {
                            unsupported(expression.location, "global aggregate addresses are not supported for SIA32 yet")
                        })?;
                        self.builder.use_var(variable)
                    }
                    _ => self.compile_expr(pointer)?,
                };
                let address = if offset == 0 {
                    address
                } else {
                    let delta = self.builder.ins().iconst(types::I32, offset as i64);
                    self.builder.ins().iadd(address, delta)
                };
                Ok(self
                    .builder
                    .ins()
                    .load(ty, MemFlagsData::new(), address, 0))
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
                            "globals are not supported for SIA32 yet",
                        )
                    })?;
                    Ok(self.builder.use_var(variable))
                }
                _ => {
                    let address = self.compile_expr(pointer)?;
                    Ok(self.builder.ins().load(ty, MemFlagsData::new(), address, 0))
                }
            },
            ExprType::Negate(value) => {
                let value = self.compile_expr(value)?;
                Ok(self.builder.ins().ineg(value))
            }
            ExprType::BitwiseNot(value) => {
                let value = self.compile_expr(value)?;
                let all_ones = self.builder.ins().iconst(ty, -1);
                Ok(self.builder.ins().bxor(value, all_ones))
            }
            ExprType::Binary(operator, left, right) => {
                use saltwater_parser::data::hir::BinaryOp;
                if *operator == BinaryOp::Assign {
                    let ExprType::Deref(pointer) = &left.expr else {
                        return Err(unsupported(
                            left.location,
                            "only plain local-variable assignment is supported for SIA32",
                        ));
                    };
                    let value = self.compile_expr(right)?;
                    if let ExprType::Id(symbol) = &pointer.expr {
                        if let Some(variable) = self.variables.get(symbol).copied() {
                            self.builder.def_var(variable, value);
                            return Ok(value);
                        }
                    }
                    let address = self.compile_expr(pointer)?;
                    self.builder
                        .ins()
                        .store(MemFlagsData::new(), value, address, 0);
                    return Ok(value);
                }
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;
                let value = match operator {
                    BinaryOp::Add => self.builder.ins().iadd(left, right),
                    BinaryOp::Sub => self.builder.ins().isub(left, right),
                    BinaryOp::BitwiseAnd => self.builder.ins().band(left, right),
                    BinaryOp::BitwiseOr => self.builder.ins().bor(left, right),
                    BinaryOp::Xor => self.builder.ins().bxor(left, right),
                    BinaryOp::Shl => self.builder.ins().ishl(left, right),
                    BinaryOp::Shr => self.builder.ins().sshr(left, right),
                    BinaryOp::Compare(compare) => {
                        use saltwater_parser::data::lex::ComparisonToken;
                        let condition = match compare {
                            ComparisonToken::Less => IntCC::SignedLessThan,
                            ComparisonToken::Greater => IntCC::SignedGreaterThan,
                            ComparisonToken::EqualEqual => IntCC::Equal,
                            ComparisonToken::NotEqual => IntCC::NotEqual,
                            ComparisonToken::LessEqual => IntCC::SignedLessThanOrEqual,
                            ComparisonToken::GreaterEqual => IntCC::SignedGreaterThanOrEqual,
                        };
                        let compared = self.builder.ins().icmp(condition, left, right);
                        self.builder.ins().uextend(ty, compared)
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
            other => Err(unsupported(
                expression.location,
                format!("SIA32 expression lowering is not implemented for {other:?}"),
            )),
        }
    }
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
