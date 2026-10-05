//! AMD64 ELF64 object emission using the maintained typed C frontend and current
//! Cranelift. Native System V scalar, variadic and naturally laid out aggregate ABI.
mod abi;
mod aggregate;
mod expr;
mod ssa;
mod stdarg;
mod stmt;
mod variadic;
use cranelift_codegen::{
    ir::{
        self, types, AbiParam, Block, Function, InstBuilder, MemFlagsData, Signature, StackSlot,
        StackSlotData, StackSlotKind, UserFuncName, Value,
    },
    isa,
    settings::{self, Configurable, Flags},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use saltwater_parser::data::{
    hir::{Declaration, Expr, ExprType, Initializer, LiteralValue, Stmt, Symbol},
    types::{ArrayType, FunctionType},
    StorageClass, Type,
};
use saltwater_parser::{check_semantics, Opt, TargetDataModel};
use std::collections::HashMap;
#[derive(Debug)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
fn err(value: impl std::fmt::Display) -> Error {
    Error(value.to_string())
}
fn unsupported(value: impl std::fmt::Display) -> Error {
    Error(format!("AMD64 lowering: {value}"))
}
pub fn configure_options(opt: &mut Opt) {
    opt.target = TargetDataModel::Amd64;
}
fn size(ty: &Type) -> Result<u64, Error> {
    ty.sizeof_for(TargetDataModel::Amd64).map_err(err)
}
fn align(ty: &Type) -> Result<u64, Error> {
    ty.alignof_for(TargetDataModel::Amd64).map_err(err)
}
fn address_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Array(..) | Type::Struct(_) | Type::Union(_) | Type::Function(_)
    )
}
fn signed(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Char(true)
            | Type::SignedChar
            | Type::Short(true)
            | Type::Int(true)
            | Type::Long(true)
            | Type::LongLong(true)
            | Type::Enum(..)
    )
}
fn ir_type(ty: &Type) -> Result<ir::Type, Error> {
    Ok(match ty {
        Type::Bool | Type::Char(_) | Type::SignedChar => types::I8,
        Type::Short(_) => types::I16,
        Type::Int(_) | Type::Enum(..) => types::I32,
        Type::Long(_) | Type::LongLong(_) | Type::Pointer(..) => types::I64,
        Type::Float => types::F32,
        Type::Double => types::F64,
        Type::LongDouble => return Err(unsupported("AMD64 long double/x87 ABI is unsupported")),
        _ if address_type(ty) => types::I64,
        _ => return Err(unsupported(format!("value type {ty:?}"))),
    })
}
fn params(ft: &FunctionType) -> &[Symbol] {
    if ft.params.len() == 1 && ft.params[0].get().ctype == Type::Void {
        &[]
    } else {
        &ft.params
    }
}
fn abi(ty: &Type) -> Result<AbiParam, Error> {
    if address_type(ty) && !matches!(ty, Type::Function(_) | Type::Array(..)) {
        return aggregate::parameter(ty);
    }
    let mut a = AbiParam::new(ir_type(ty)?);
    if a.value_type.is_int() && a.value_type.bits() < 32 {
        a.extension = if signed(ty) {
            ir::ArgumentExtension::Sext
        } else {
            ir::ArgumentExtension::Uext
        };
    }
    Ok(a)
}
fn signature(ft: &FunctionType, checked: bool) -> Result<Signature, Error> {
    Ok(abi::plan(ft, &[], checked)?.signature)
}
#[derive(Clone, Copy)]
enum Id {
    Function(FuncId),
    Data(DataId),
}
struct Compiler {
    module: ObjectModule,
    ids: HashMap<Symbol, Id>,
    strings: HashMap<Vec<u8>, DataId>,
    static_count: usize,
}
impl Compiler {
    fn declare(&mut self, decl: &Declaration) -> Result<(), Error> {
        let m = decl.symbol.get();
        if m.storage_class == StorageClass::Typedef {
            return Ok(());
        }
        let name = m.id.resolve_and_clone();
        let definition = decl.init.is_some()
            || m.storage_class != StorageClass::Extern && !matches!(m.ctype, Type::Function(_));
        let linkage = if m.storage_class == StorageClass::Static {
            Linkage::Local
        } else if definition {
            Linkage::Export
        } else {
            Linkage::Import
        };
        let id = match &m.ctype {
            Type::Function(ft) => Id::Function(
                self.module
                    .declare_function(&name, linkage, &signature(ft, false)?)
                    .map_err(err)?,
            ),
            _ => Id::Data(
                self.module
                    .declare_data(&name, linkage, !m.qualifiers.c_const, false)
                    .map_err(err)?,
            ),
        };
        self.ids.insert(decl.symbol, id);
        Ok(())
    }
    fn string(&mut self, bytes: &[u8]) -> Result<DataId, Error> {
        if let Some(id) = self.strings.get(bytes) {
            return Ok(*id);
        }
        let id = self
            .module
            .declare_data(
                &format!(".cosmic.str.{}", self.strings.len()),
                Linkage::Local,
                false,
                false,
            )
            .map_err(err)?;
        let mut data = DataDescription::new();
        data.set_align(4); // also satisfies UTF-32 wchar_t literal alignment
        data.define(bytes.to_vec().into_boxed_slice());
        self.module.define_data(id, &data).map_err(err)?;
        self.strings.insert(bytes.to_vec(), id);
        Ok(id)
    }
    fn static_ref(&mut self, e: &Expr) -> Result<Option<(Id, i64)>, Error> {
        match &e.expr {
            ExprType::Cast(v) | ExprType::Noop(v) => self.static_ref(v),
            ExprType::StaticRef(v) => match &v.expr {
                ExprType::Id(s) => Ok(self.ids.get(s).copied().map(|id| (id, 0))),
                ExprType::Member(base, field) => {
                    if let Some((id, add)) = self.static_ref(&Expr {
                        expr: ExprType::StaticRef(base.clone()),
                        ..e.clone()
                    })? {
                        Ok(Some((id, add + member_offset(&base.ctype, *field)? as i64)))
                    } else {
                        Ok(None)
                    }
                }
                _ => self.static_ref(v),
            },
            ExprType::Id(s) if address_type(&s.get().ctype) => {
                Ok(self.ids.get(s).copied().map(|id| (id, 0)))
            }
            ExprType::Binary(
                saltwater_parser::data::hir::BinaryOp::Add
                | saltwater_parser::data::hir::BinaryOp::Sub,
                base,
                delta,
            ) => {
                let Some((id, addend)) = self.static_ref(base)? else {
                    return Ok(None);
                };
                let delta = delta
                    .clone()
                    .const_fold_for(TargetDataModel::Amd64)
                    .map_err(|e| err(e.data))?;
                let value = match delta.expr {
                    ExprType::Literal(LiteralValue::Int(v)) => v,
                    ExprType::Literal(LiteralValue::UnsignedInt(v)) => {
                        i64::try_from(v).map_err(err)?
                    }
                    ExprType::Literal(LiteralValue::Char(v)) => v as i64,
                    _ => return Ok(None),
                };
                let subtract = matches!(
                    e.expr,
                    ExprType::Binary(saltwater_parser::data::hir::BinaryOp::Sub, _, _)
                );
                let addend = if subtract {
                    addend.checked_sub(value)
                } else {
                    addend.checked_add(value)
                }
                .ok_or_else(|| unsupported("static address addend overflow"))?;
                Ok(Some((id, addend)))
            }
            ExprType::Literal(LiteralValue::Str(b)) => Ok(Some((Id::Data(self.string(b)?), 0))),
            _ => Ok(None),
        }
    }
    fn initialize(
        &mut self,
        desc: &mut DataDescription,
        bytes: &mut [u8],
        off: usize,
        ty: &Type,
        init: &Initializer,
    ) -> Result<(), Error> {
        match init {
            Initializer::Zero => Ok(()),
            Initializer::Scalar(e) => {
                if matches!(ty, Type::Pointer(..)) {
                    if let Some((id, add)) = self.static_ref(e)? {
                        match id {
                            Id::Function(f) => {
                                if add != 0 {
                                    return Err(unsupported("function pointer initializer addend"));
                                }
                                let r = self.module.declare_func_in_data(f, desc);
                                desc.write_function_addr(off as u32, r);
                            }
                            Id::Data(d) => {
                                let r = self.module.declare_data_in_data(d, desc);
                                desc.write_data_addr(off as u32, r, add);
                            }
                        }
                        return Ok(());
                    }
                }
                if let (Type::Array(element, _), ExprType::Literal(LiteralValue::Str(b))) =
                    (ty, &e.expr)
                {
                    if string_array_element(element, &e.ctype) {
                        let n = b.len().min(size(ty)? as usize);
                        bytes[off..off + n].copy_from_slice(&b[..n]);
                        return Ok(());
                    }
                }
                if static_zero(e) {
                    return Ok(());
                }
                let folded = e
                    .clone()
                    .const_fold_for(TargetDataModel::Amd64)
                    .map_err(|e| err(e.data))?;
                let width = size(ty)? as usize;
                let raw = match folded.expr {
                    ExprType::Literal(LiteralValue::Int(v)) => v.to_le_bytes().to_vec(),
                    ExprType::Literal(LiteralValue::UnsignedInt(v)) => v.to_le_bytes().to_vec(),
                    ExprType::Literal(LiteralValue::Char(v)) => (v as u64).to_le_bytes().to_vec(),
                    ExprType::Literal(LiteralValue::Float(v)) => {
                        if *ty == Type::Float {
                            (v as f32).to_bits().to_le_bytes().to_vec()
                        } else {
                            v.to_bits().to_le_bytes().to_vec()
                        }
                    }
                    _ => return Err(unsupported("nonconstant static initializer")),
                };
                if width > raw.len() {
                    return Err(unsupported("static initializer representation"));
                }
                bytes[off..off + width].copy_from_slice(&raw[..width]);
                Ok(())
            }
            Initializer::InitializerList(items) => {
                for (at, item_type, item) in initializer_items(ty, items)? {
                    self.initialize(desc, bytes, off + at, &item_type, item)?;
                }
                Ok(())
            }
            _ => Err(unsupported("invalid data initializer")),
        }
    }
}
fn string_array_element(element: &Type, source: &Type) -> bool {
    matches!(element, Type::Char(_) | Type::SignedChar)
        || (matches!(element, Type::Int(_))
            && matches!(source,Type::Array(s,_) if s.as_ref()==element))
}
fn static_zero(e: &Expr) -> bool {
    match &e.expr {
        ExprType::Literal(
            LiteralValue::Int(0) | LiteralValue::UnsignedInt(0) | LiteralValue::Char(0),
        ) => true,
        ExprType::Cast(v) | ExprType::Noop(v) | ExprType::StaticRef(v) => static_zero(v),
        _ => false,
    }
}

fn member_offset(ty: &Type, name: saltwater_parser::intern::InternedStr) -> Result<u64, Error> {
    match ty {
        Type::Struct(s) => Ok(s.offset_for(name, TargetDataModel::Amd64)),
        Type::Union(_) => Ok(0),
        _ => Err(unsupported("member address on nonaggregate")),
    }
}
fn initializer_items<'a>(
    ty: &Type,
    items: &'a [Initializer],
) -> Result<Vec<(usize, Type, &'a Initializer)>, Error> {
    let mut out = Vec::new();
    match ty {
        Type::Array(element, ArrayType::Fixed(count)) => {
            if items.len() > *count as usize {
                return Err(unsupported("too many array initializers"));
            }
            for (i, item) in items.iter().enumerate() {
                out.push((i * size(element)? as usize, (**element).clone(), item));
            }
        }
        Type::Struct(s) => {
            let mut off = 0u64;
            for (field, item) in s.members().iter().zip(items) {
                let a = align(&field.ctype)?;
                off = (off + a - 1) & !(a - 1);
                out.push((off as usize, field.ctype.clone(), item));
                off += size(&field.ctype)?;
            }
        }
        Type::Union(s) => {
            if let Some((field, item)) = s.members().first().zip(items.first()) {
                out.push((0, field.ctype.clone(), item));
            }
        }
        _ => {
            if items.len() != 1 {
                return Err(unsupported("scalar initializer list"));
            }
            out.push((0, ty.clone(), &items[0]));
        }
    }
    Ok(out)
}
/// Code generation policy; these levels do not imply GCC/LLVM optimization parity.
#[derive(Clone, Copy, Debug, Default)]
pub enum Optimization {
    #[default]
    None,
    Speed,
    SpeedAndSize,
}
pub fn compile(source: &str, opt: Opt) -> Result<Vec<u8>, Error> {
    compile_with_optimization(source, opt, Optimization::None)
}
pub fn compile_with_optimization(
    source: &str,
    mut opt: Opt,
    optimization: Optimization,
) -> Result<Vec<u8>, Error> {
    configure_options(&mut opt);
    let declarations = check_semantics(source, opt).result.map_err(|errors| {
        err(errors
            .iter()
            .map(|e| e.data.to_string())
            .collect::<Vec<_>>()
            .join("\n"))
    })?;
    let mut flags = settings::builder();
    flags
        .set(
            "opt_level",
            match optimization {
                Optimization::None => "none",
                Optimization::Speed => "speed",
                Optimization::SpeedAndSize => "speed_and_size",
            },
        )
        .map_err(err)?;
    flags.enable("is_pic").map_err(err)?;
    flags.enable("enable_verifier").map_err(err)?;
    flags.set("enable_probestack", "false").map_err(err)?;
    if declarations.iter().any(|d| {
        matches!(&d.data.symbol.get().ctype, Type::Function(ft) if ft.varargs)
            && matches!(&d.data.init, Some(Initializer::FunctionBody(_)))
    }) {
        flags.set("preserve_frame_pointers", "true").map_err(err)?;
    }
    let triple = "x86_64-unknown-linux-gnu".parse().map_err(err)?;
    let isa = isa::lookup(triple)
        .map_err(err)?
        .finish(Flags::new(flags))
        .map_err(err)?;
    let module = ObjectModule::new(
        ObjectBuilder::new(
            isa,
            "cosmic-amd64",
            cranelift_module::default_libcall_names(),
        )
        .map_err(err)?,
    );
    let mut c = Compiler {
        module,
        ids: HashMap::new(),
        strings: HashMap::new(),
        static_count: 0,
    };
    for d in &declarations {
        c.declare(&d.data)?;
    }
    // Coalesce tentative declarations with an eventual initializer. ELF has
    // one definition per translation-unit symbol, including repeated externs.
    let mut global_definitions = HashMap::new();
    for (index, d) in declarations.iter().enumerate() {
        let m = d.data.symbol.get();
        if m.storage_class != StorageClass::Typedef
            && !matches!(m.ctype, Type::Function(_))
            && (m.storage_class != StorageClass::Extern || d.data.init.is_some())
        {
            if let Some(Id::Data(id)) = c.ids.get(&d.data.symbol) {
                if d.data.init.is_some() || !global_definitions.contains_key(id) {
                    global_definitions.insert(*id, index);
                }
            }
        }
    }
    for (declaration_index, d) in declarations.iter().enumerate() {
        let m = d.data.symbol.get();
        if m.storage_class == StorageClass::Typedef {
            continue;
        }
        match &m.ctype {
            Type::Function(ft) => {
                if let Some(Initializer::FunctionBody(body)) = &d.data.init {
                    let Id::Function(id) = c.ids[&d.data.symbol] else {
                        unreachable!()
                    };
                    let mut ctx = c.module.make_context();
                    ctx.func = Function::with_name_signature(
                        UserFuncName::user(0, id.as_u32()),
                        stdarg::incoming_signature(ft)?,
                    );
                    let mut fbctx = FunctionBuilderContext::new();
                    {
                        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
                        let entry = b.create_block();
                        b.append_block_params_for_function_params(entry);
                        b.switch_to_block(entry);
                        let variables = ssa::variables(&mut b, ft, body);
                        let mut f = Lowerer {
                            c: &mut c,
                            b: &mut b,
                            locals: HashMap::new(),
                            referenced: ssa::referenced(body),
                            variables,
                            loops: Vec::new(),
                            breaks: Vec::new(),
                            labels: HashMap::new(),
                            switches: Vec::new(),
                            terminated: false,
                            return_type: (*ft.return_type).clone(),
                            incoming_variadic: None,
                            aggregate_return: None,
                            return_classes: None,
                        };
                        let values = f.b.block_params(entry).to_vec();
                        let plan = abi::plan(ft, &[], true)?;
                        let hidden_return = matches!(plan.result, abi::ResultKind::Memory);
                        f.aggregate_return = hidden_return.then(|| values[0]);
                        f.return_classes = if let abi::ResultKind::Registers(c) = &plan.result {
                            Some(c.clone())
                        } else {
                            None
                        };
                        f.capture_variadic(ft, &values)?;
                        let mut index = usize::from(hidden_return);
                        for (p, passing) in params(ft).iter().zip(&plan.arguments) {
                            let ty = &p.get().ctype;
                            let count = if let abi::Passing::Registers(c) = passing {
                                c.len()
                            } else {
                                1
                            };
                            let incoming = &values[index..index + count];
                            index += count;
                            let v = if matches!(passing, abi::Passing::Registers(_)) {
                                f.aggregate_from_registers(ty, incoming)?
                            } else {
                                incoming[0]
                            };
                            if let Some(var) = f.variables.get(p).copied() {
                                f.b.def_var(var, v);
                            } else {
                                let a = f.local(*p, ty)?;
                                if matches!(ty, Type::Struct(_) | Type::Union(_)) {
                                    let bytes = f.b.ins().iconst(types::I64, size(ty)? as i64);
                                    f.b.call_memmove(f.c.module.target_config(), a, v, bytes);
                                } else {
                                    f.b.ins().store(MemFlagsData::new(), v, a, 0);
                                }
                            }
                        }
                        for stmt in body {
                            f.preallocate_locals(stmt)?;
                        }
                        for stmt in body {
                            f.stmt(stmt)?;
                        }
                        if !f.terminated {
                            if *ft.return_type == Type::Void {
                                f.b.ins().return_(&[]);
                            } else if let Some(classes) = f.return_classes.clone() {
                                let zeros = classes
                                    .iter()
                                    .map(|t| {
                                        if t.is_float() {
                                            f.b.ins().f64const(0.0)
                                        } else {
                                            f.b.ins().iconst(*t, 0)
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                f.b.ins().return_(&zeros);
                            } else {
                                // Reaching a nonvoid closing brace is valid C
                                // when the caller discards the result. A caller
                                // that uses it has undefined behavior (except
                                // main, whose result is specified as zero).
                                let ty = ir_type(&ft.return_type)?;
                                let zero = if let Some(destination) = f.aggregate_return {
                                    destination
                                } else if ty == types::F32 {
                                    f.b.ins().f32const(0.0)
                                } else if ty == types::F64 {
                                    f.b.ins().f64const(0.0)
                                } else {
                                    f.b.ins().iconst(ty, 0)
                                };
                                f.b.ins().return_(&[zero]);
                            }
                        }
                        b.seal_all_blocks();
                        b.finalize(c.module.target_config());
                    }
                    c.module.define_function(id, &mut ctx).map_err(|e| {
                        unsupported(format!("function {}: {e:?}\n{}", m.id, ctx.func))
                    })?;
                }
            }
            _ => {
                if m.storage_class != StorageClass::Extern || d.data.init.is_some() {
                    let Id::Data(id) = c.ids[&d.data.symbol] else {
                        unreachable!()
                    };
                    if global_definitions.get(&id) != Some(&declaration_index) {
                        continue;
                    }
                    let mut desc = DataDescription::new();
                    desc.set_align(align(&m.ctype)?);
                    let mut bytes = vec![0; size(&m.ctype)? as usize];
                    if let Some(init) = &d.data.init {
                        c.initialize(&mut desc, &mut bytes, 0, &m.ctype, init)?;
                    }
                    desc.define(bytes.into_boxed_slice());
                    c.module.define_data(id, &desc).map_err(err)?;
                }
            }
        }
    }
    c.module.finish().emit().map_err(err)
}
struct Lowerer<'a, 'b> {
    c: &'a mut Compiler,
    b: &'a mut FunctionBuilder<'b>,
    locals: HashMap<Symbol, StackSlot>,
    referenced: std::collections::HashSet<Symbol>,
    variables: HashMap<Symbol, cranelift_frontend::Variable>,
    loops: Vec<(Block, Block)>,
    breaks: Vec<Block>,
    labels: HashMap<saltwater_parser::intern::InternedStr, Block>,
    switches: Vec<(HashMap<u64, Block>, Block)>,
    terminated: bool,
    return_type: Type,
    incoming_variadic: Option<stdarg::Incoming>,
    aggregate_return: Option<Value>,
    return_classes: Option<Vec<ir::Type>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_elf64_with_lp64_scalars_and_external_relocations() {
        let bytes = compile("extern long external(long); long values[2]={0x123456789abcdef0L,-7L}; long *pointer=values; long invoke(long x) { return external(x)+pointer[1]+sizeof(long)+sizeof(void*); }", Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
        assert_eq!(bytes[4], 2);
        assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 62);
        assert!(bytes
            .windows(8)
            .any(|w| w == 0x123456789abcdef0u64.to_le_bytes()));
        assert!(bytes.windows(8).any(|w| w == b"external"));
    }
    #[test]
    fn unused_unsupported_abi_prototypes_do_not_block_scalar_code() {
        compile("struct S { long x; }; struct S declaration(struct S); int variadic(int x,...); int main(void) { return 0; }", Opt::default()).unwrap();
        compile(
            "int variadic(int x,...); int main(void) { return variadic(1,2); }",
            Opt::default(),
        )
        .unwrap();
    }
    #[test]
    fn compiles_control_flow_and_native_floating_point() {
        compile("double sum(double x) { double a[2]={x,2.5}; int i; for(i=0;i<2;i++) x += a[i]; if(x>0.0) return x; return -x; } int branch(int x) { switch(x) { case 1:return 7; case 2: x=8; break; default:x=9; } return x; }", Opt::default()).unwrap();
    }
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn scalar_ssa_preserves_control_flow_and_escaped_storage() {
        let source = r#"
        int mutate(int *p) { *p += 3; return *p; }
        int main(void) {
            int x=0, sum=0, i=0; volatile int observed=2;
            while(i<10) { ++i; if(i==3) continue; sum+=i; if(i==8) break; }
            if(sum!=33 || i!=8) return 1;
            do { x++; } while(x<3);
            for(int j=0;j<4;j++) { int x=j; sum+=x; }
            if(sum!=39 || x!=3) return 2;
            switch(x) {case 3: x=7; break; default: x=99;}
            if(x!=7) return 3;
            i=0; again: ++i; if(i<4) goto again;
            if(i!=4) return 4;
            x=0; if((i==4 && (x=5)) || (x=99)) sum+=x;
            if(x!=5 || sum!=44) return 5;
            x=(i==4 ? (sum=6) : (sum=8));
            if(x!=6 || sum!=6) return 6;
            if(mutate(&x)!=9 || x!=9) return 7;
            observed+=x; if(observed!=11) return 8;
            unsigned char narrow=255; narrow++; if(narrow!=0) return 9;
            double value=1.5; value+=2.0; if(value!=3.5) return 10;
            int *pointer=&sum; (*pointer)++; if(sum!=7) return 11;
            return 0;
        }"#;
        for optimization in [
            Optimization::None,
            Optimization::Speed,
            Optimization::SpeedAndSize,
        ] {
            let object = compile_with_optimization(source, Opt::default(), optimization).unwrap();
            let dir = std::env::temp_dir().join(format!(
                "cosmic-ssa-{}-{:?}",
                std::process::id(),
                optimization
            ));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("guest.o"), object).unwrap();
            let output = std::process::Command::new("cc")
                .arg(dir.join("guest.o"))
                .arg("-o")
                .arg(dir.join("guest"))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let status = std::process::Command::new(dir.join("guest"))
                .status()
                .unwrap();
            assert!(status.success(), "SSA fixture exit {status}");
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn links_and_executes_aggregate_copies_with_exact_self_aliasing() {
        let source = "struct S { char c; long x; long *p; }; int main(void) { struct S a={7,0x100000001L,0}; struct S b=a; b=b; b=a; if(sizeof(a)!=24 || b.x!=0x100000001L || b.c!=7 || b.p!=0) return 1; return 0; }";
        for optimization in [
            Optimization::None,
            Optimization::Speed,
            Optimization::SpeedAndSize,
        ] {
            let object = compile_with_optimization(source, Opt::default(), optimization).unwrap();
            let directory = std::env::temp_dir().join(format!(
                "cosmic-amd64-aggregate-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&directory).unwrap();
            let path = directory.join("guest.o");
            let executable = directory.join("guest");
            std::fs::write(&path, object).unwrap();
            let linked = std::process::Command::new("cc")
                .arg(&path)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                linked.status.success(),
                "{}",
                String::from_utf8_lossy(&linked.stderr)
            );
            assert!(std::process::Command::new(&executable)
                .status()
                .unwrap()
                .success());
            std::fs::remove_dir_all(directory).unwrap();
        }
    }
}
