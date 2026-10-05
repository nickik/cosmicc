use super::*;
use cranelift_frontend::Switch;
use saltwater_parser::data::hir::StmtType;
impl Lowerer<'_, '_> {
    pub(super) fn preallocate_locals(&mut self, statement: &Stmt) -> Result<(), Error> {
        match &statement.data {
            StmtType::Decl(declarations) => {
                for declaration in declarations {
                    let symbol = declaration.data.symbol;
                    let meta = symbol.get();
                    if matches!(
                        meta.storage_class,
                        StorageClass::Auto | StorageClass::Register
                    ) && !self.variables.contains_key(&symbol)
                        && size(&meta.ctype).is_ok()
                    {
                        self.local(symbol, &meta.ctype)?;
                    }
                }
            }
            StmtType::Compound(body) => {
                for s in body {
                    self.preallocate_locals(s)?;
                }
            }
            StmtType::If(_, a, b) => {
                self.preallocate_locals(a)?;
                if let Some(b) = b {
                    self.preallocate_locals(b)?;
                }
            }
            StmtType::For(a, _, _, b) => {
                self.preallocate_locals(a)?;
                self.preallocate_locals(b)?;
            }
            StmtType::Do(a, _)
            | StmtType::While(_, a)
            | StmtType::Switch(_, a)
            | StmtType::Label(_, a)
            | StmtType::Case(_, a)
            | StmtType::Default(a) => self.preallocate_locals(a)?,
            _ => {}
        }
        Ok(())
    }
    fn jump(&mut self, block: Block) {
        if !self.terminated {
            self.b.ins().jump(block, &[]);
        }
        self.terminated = true;
    }
    fn enter(&mut self, block: Block) {
        self.b.switch_to_block(block);
        self.terminated = false;
    }
    fn scalar_initializer(&mut self, ty: &Type, init: &Initializer) -> Result<Value, Error> {
        let t = ir_type(ty)?;
        match init {
            Initializer::Scalar(e) => {
                let v = if e.lval && matches!(ty,Type::Pointer(p,_)if p.as_ref()==&e.ctype) {
                    self.address(e)?
                } else {
                    self.expr(e)?
                };
                Ok(self.cast(v, t, &e.ctype))
            }
            Initializer::InitializerList(v) if v.len() == 1 => self.scalar_initializer(ty, &v[0]),
            Initializer::Zero => Ok(if t == types::F32 {
                self.b.ins().f32const(0.0)
            } else if t == types::F64 {
                self.b.ins().f64const(0.0)
            } else {
                self.b.ins().iconst(t, 0)
            }),
            _ => Err(unsupported("invalid scalar initializer")),
        }
    }
    fn initialize_local(&mut self, a: Value, ty: &Type, init: &Initializer) -> Result<(), Error> {
        match init {
            Initializer::Zero => {
                for i in 0..size(ty)? {
                    let z = self.b.ins().iconst(types::I8, 0);
                    self.b
                        .ins()
                        .store(MemFlagsData::new(), z, a, i32::try_from(i).map_err(err)?);
                }
                Ok(())
            }
            Initializer::Scalar(e) => {
                if let (Type::Array(element, _), ExprType::Literal(LiteralValue::Str(bytes))) =
                    (ty, &e.expr)
                {
                    if string_array_element(element, &e.ctype) {
                        for i in 0..size(ty)? as usize {
                            let v = self
                                .b
                                .ins()
                                .iconst(types::I8, bytes.get(i).copied().unwrap_or(0) as i64);
                            self.b.ins().store(MemFlagsData::new(), v, a, i as i32);
                        }
                        return Ok(());
                    }
                }
                if matches!(ty, Type::Struct(_) | Type::Union(_)) {
                    let src = self.expr(e)?;
                    let count = self.b.ins().iconst(types::I64, size(ty)? as i64);
                    self.b
                        .call_memmove(self.c.module.target_config(), a, src, count);
                    return Ok(());
                }
                if address_type(ty) {
                    return Err(unsupported("invalid array copy initializer"));
                }
                let v = if e.lval && matches!(ty,Type::Pointer(p,_)if p.as_ref()==&e.ctype) {
                    self.address(e)?
                } else {
                    self.expr(e)?
                };
                let v = self.cast(v, ir_type(ty)?, &e.ctype);
                self.b.ins().store(MemFlagsData::new(), v, a, 0);
                Ok(())
            }
            Initializer::InitializerList(items) => {
                self.initialize_local(a, ty, &Initializer::Zero)?;
                for (off, itemty, item) in initializer_items(ty, items)? {
                    let ptr = self.b.ins().iadd_imm_s(a, off as i64);
                    self.initialize_local(ptr, &itemty, item)?;
                }
                Ok(())
            }
            _ => Err(unsupported("invalid local initializer")),
        }
    }
    pub(super) fn stmt(&mut self, s: &Stmt) -> Result<(), Error> {
        if self.terminated
            && !matches!(
                &s.data,
                StmtType::Label(..)
                    | StmtType::Case(..)
                    | StmtType::Default(..)
                    | StmtType::Compound(_)
            )
        {
            return Ok(());
        }
        match &s.data {
            StmtType::Compound(body) => {
                for s in body {
                    self.stmt(s)?;
                }
            }
            StmtType::Expr(e) => {
                self.expr(e)?;
            }
            StmtType::Decl(decls) => {
                for d in decls {
                    let meta = d.data.symbol.get();
                    fn variable_type(ty: &Type) -> bool {
                        match ty {
                            Type::Array(t, b) => {
                                matches!(b, ArrayType::Variable(_)) || variable_type(t)
                            }
                            Type::Pointer(t, _) => variable_type(t),
                            _ => false,
                        }
                    }
                    if meta.storage_class == StorageClass::Typedef {
                        if variable_type(&meta.ctype) {
                            return Err(unsupported(
                                "variably modified typedef bound capture is unsupported",
                            ));
                        }
                        continue;
                    }
                    if meta.storage_class == StorageClass::Extern
                        || matches!(meta.ctype, Type::Function(_))
                    {
                        self.c.declare(&d.data)?;
                        continue;
                    }
                    if meta.storage_class == StorageClass::Static {
                        let name = format!(".cosmic.static.{}", self.c.static_count);
                        self.c.static_count += 1;
                        let id = self
                            .c
                            .module
                            .declare_data(&name, Linkage::Local, !meta.qualifiers.c_const, false)
                            .map_err(err)?;
                        self.c.ids.insert(d.data.symbol, Id::Data(id));
                        let mut desc = DataDescription::new();
                        desc.set_align(align(&meta.ctype)?);
                        let mut bytes = vec![0; size(&meta.ctype)? as usize];
                        if let Some(init) = &d.data.init {
                            self.c
                                .initialize(&mut desc, &mut bytes, 0, &meta.ctype, init)?;
                        }
                        desc.define(bytes.into_boxed_slice());
                        self.c.module.define_data(id, &desc).map_err(err)?;
                        continue;
                    }
                    if matches!(meta.ctype, Type::Pointer(..)) && variable_type(&meta.ctype) {
                        return Err(unsupported(
                            "variably modified pointer bound capture is unsupported",
                        ));
                    }
                    if !self.referenced.contains(&d.data.symbol) && d.data.init.is_none() {
                        fn evaluate_bounds(
                            f: &mut Lowerer<'_, '_>,
                            ty: &Type,
                        ) -> Result<bool, Error> {
                            if let Type::Array(element, length) = ty {
                                let nested = evaluate_bounds(f, element)?;
                                if let ArrayType::Variable(bound) = length {
                                    f.expr(bound)?;
                                    return Ok(true);
                                }
                                return Ok(nested);
                            }
                            Ok(false)
                        }
                        // An unused automatic VLA has no observable storage
                        // effects, but its bound expressions must execute.
                        if evaluate_bounds(self, &meta.ctype)? {
                            continue;
                        }
                    }
                    if let Some(var) = self.variables.get(&d.data.symbol).copied() {
                        if let Some(init) = &d.data.init {
                            let v = self.scalar_initializer(&meta.ctype, init)?;
                            self.b.def_var(var, v);
                        }
                        continue;
                    }
                    let a = self.local(d.data.symbol, &meta.ctype)?;
                    if let Some(init) = &d.data.init {
                        self.initialize_local(a, &meta.ctype, init)?;
                    }
                }
            }
            StmtType::Return(e) => {
                if let Some(e) = e {
                    let v = self.expr(e)?;
                    if let Some(classes) = self.return_classes.clone() {
                        let address = self.aggregate_snapshot(v, &self.return_type.clone())?;
                        let values = classes
                            .iter()
                            .enumerate()
                            .map(|(i, t)| {
                                self.b
                                    .ins()
                                    .load(*t, MemFlagsData::new(), address, (i * 8) as i32)
                            })
                            .collect::<Vec<_>>();
                        self.b.ins().return_(&values);
                    } else if let Some(destination) = self.aggregate_return {
                        let bytes = self
                            .b
                            .ins()
                            .iconst(types::I64, size(&self.return_type)? as i64);
                        self.b
                            .call_memmove(self.c.module.target_config(), destination, v, bytes);
                        self.b.ins().return_(&[destination]);
                    } else if self.return_type == Type::Void {
                        self.b.ins().return_(&[]);
                    } else {
                        let v = self.cast(v, ir_type(&self.return_type)?, &e.ctype);
                        self.b.ins().return_(&[v]);
                    }
                } else {
                    self.b.ins().return_(&[]);
                }
                self.terminated = true;
            }
            StmtType::If(cond, yes, no) => {
                let c = self.condition(cond)?;
                let y = self.b.create_block();
                let n = self.b.create_block();
                let end = self.b.create_block();
                self.b.ins().brif(c, y, &[], n, &[]);
                self.enter(y);
                self.stmt(yes)?;
                let yt = self.terminated;
                self.jump(end);
                self.enter(n);
                if let Some(no) = no {
                    self.stmt(no)?;
                }
                let nt = self.terminated;
                self.jump(end);
                if yt && nt {
                    self.terminated = true;
                } else {
                    self.enter(end);
                }
            }
            StmtType::While(cond, body) | StmtType::Do(body, cond) => {
                let test = self.b.create_block();
                let work = self.b.create_block();
                let end = self.b.create_block();
                let do_first = matches!(&s.data, StmtType::Do(..));
                self.jump(if do_first { work } else { test });
                self.loops.push((test, end));
                self.breaks.push(end);
                self.enter(work);
                self.stmt(body)?;
                self.jump(test);
                self.enter(test);
                let c = self.condition(cond)?;
                self.b.ins().brif(c, work, &[], end, &[]);
                self.loops.pop();
                self.breaks.pop();
                self.enter(end);
            }
            StmtType::For(init, cond, increment, body) => {
                self.stmt(init)?;
                let test = self.b.create_block();
                let work = self.b.create_block();
                let inc = self.b.create_block();
                let end = self.b.create_block();
                self.jump(test);
                self.enter(test);
                if let Some(cond) = cond {
                    let c = self.condition(cond)?;
                    self.b.ins().brif(c, work, &[], end, &[]);
                } else {
                    self.b.ins().jump(work, &[]);
                }
                self.loops.push((inc, end));
                self.breaks.push(end);
                self.enter(work);
                self.stmt(body)?;
                self.jump(inc);
                self.enter(inc);
                if let Some(e) = increment {
                    self.expr(e)?;
                }
                self.jump(test);
                self.loops.pop();
                self.breaks.pop();
                self.enter(end);
            }
            StmtType::Continue => self.jump(
                self.loops
                    .last()
                    .ok_or_else(|| unsupported("continue outside loop"))?
                    .0,
            ),
            StmtType::Break => self.jump(
                *self
                    .breaks
                    .last()
                    .ok_or_else(|| unsupported("break outside loop/switch"))?,
            ),
            StmtType::Goto(name) => {
                let block = if let Some(b) = self.labels.get(name) {
                    *b
                } else {
                    let b = self.b.create_block();
                    self.labels.insert(*name, b);
                    b
                };
                self.jump(block);
            }
            StmtType::Label(name, body) => {
                let block = if let Some(b) = self.labels.get(name) {
                    *b
                } else {
                    let b = self.b.create_block();
                    self.labels.insert(*name, b);
                    b
                };
                self.jump(block);
                self.enter(block);
                self.stmt(body)?;
            }
            StmtType::Switch(value, body) => {
                let v = self.expr(value)?;
                let end = self.b.create_block();
                let mut cases = HashMap::new();
                let mut default = None;
                collect_cases(body, &mut cases, &mut default, self.b);
                let default = default.unwrap_or(end);
                let mut switch = Switch::new();
                for (key, block) in &cases {
                    switch.set_entry(*key as u128, *block);
                }
                switch.emit(self.b, v, default);
                self.terminated = true;
                self.breaks.push(end);
                self.switches.push((cases, default));
                self.stmt(body)?;
                self.jump(end);
                self.breaks.pop();
                self.switches.pop();
                self.enter(end);
            }
            StmtType::Case(value, body) => {
                let block = *self
                    .switches
                    .last()
                    .ok_or_else(|| unsupported("case outside switch"))?
                    .0
                    .get(value)
                    .ok_or_else(|| unsupported("missing switch case"))?;
                self.jump(block);
                self.enter(block);
                self.stmt(body)?;
            }
            StmtType::Default(body) => {
                let block = self
                    .switches
                    .last()
                    .ok_or_else(|| unsupported("default outside switch"))?
                    .1;
                self.jump(block);
                self.enter(block);
                self.stmt(body)?;
            }
        }
        Ok(())
    }
}
fn collect_cases(
    stmt: &Stmt,
    cases: &mut HashMap<u64, Block>,
    default: &mut Option<Block>,
    b: &mut FunctionBuilder<'_>,
) {
    match &stmt.data {
        StmtType::Case(v, s) => {
            cases.insert(*v, b.create_block());
            collect_cases(s, cases, default, b);
        }
        StmtType::Default(s) => {
            *default = Some(b.create_block());
            collect_cases(s, cases, default, b);
        }
        StmtType::Compound(ss) => {
            for s in ss {
                collect_cases(s, cases, default, b);
            }
        }
        StmtType::If(_, a, c) => {
            collect_cases(a, cases, default, b);
            if let Some(c) = c {
                collect_cases(c, cases, default, b);
            }
        }
        StmtType::While(_, s)
        | StmtType::Do(s, _)
        | StmtType::Label(_, s)
        | StmtType::For(_, _, _, s) => collect_cases(s, cases, default, b),
        _ => {}
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    #[test]
    fn unused_vla_bounds_and_goto_storage_emit_elf() {
        let bytes = compile(include_str!("../tools/amd64/unused-vla.c"), Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn referenced_vla_storage_remains_diagnosed() {
        let error = compile(
            "int f(int n){int data[n];data[0]=7;return data[0];}",
            Opt::default(),
        )
        .unwrap_err();
        assert!(format!("{error}").contains("variable length array"));
    }
    #[test]
    fn dynamic_typedef_bounds_are_not_silently_ignored() {
        let error =
            compile("int f(int n){typedef int A[n++];return n;}", Opt::default()).unwrap_err();
        assert!(format!("{error}").contains("typedef bound capture"));
    }
}
