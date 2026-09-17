//! Cosmic C's initial SIA32 lowering path.
//!
//! This crate deliberately emits only SIA32 code.  It does not fall back to a
//! host ISA, and it rejects floating-point C before creating Cranelift IR.

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;
use std::fmt;

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::{
    types, AbiParam, Function, InstBuilder, Signature, UserFuncName, Value,
};
use cranelift_codegen::isa::{self, CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use saltwater_parser::data::types::{FunctionType, StructType};
use saltwater_parser::data::{
    hir::{Declaration, Expr, ExprType, Initializer, LiteralValue, Stmt, StmtType, Symbol},
    CompileError, Location, StorageClass, Type,
};
use saltwater_parser::{check_semantics, Opt};
use target_lexicon::Triple;

/// The fixed target accepted by this compiler stage.
pub const TARGET: &str = "sia32-unknown-none";

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

impl Artifact {
    /// Serialize this artifact to the stable, little-endian `COSMIC-SIA` bundle format.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::from(&b"COSMIC-SIA\0"[..]);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
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
                _ => Err(unsupported(
                    expression.location,
                    "pointer dereferences are not supported for SIA32 yet",
                )),
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
                    let ExprType::Id(symbol) = &pointer.expr else {
                        return Err(unsupported(
                            left.location,
                            "only plain local-variable assignment is supported for SIA32",
                        ));
                    };
                    let variable = self.variables.get(symbol).copied().ok_or_else(|| {
                        unsupported(
                            left.location,
                            "assignment to globals is not supported for SIA32 yet",
                        )
                    })?;
                    let value = self.compile_expr(right)?;
                    self.builder.def_var(variable, value);
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
                    _ => {
                        return Err(unsupported(
                            expression.location,
                            "this C operator is not implemented in the initial SIA32 compiler path",
                        ));
                    }
                };
                Ok(value)
            }
            _ => Err(unsupported(
                expression.location,
                "this C expression is not implemented in the initial SIA32 compiler path",
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
                "this C type is not implemented for SIA32 yet",
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
