use super::*;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use saltwater_parser::data::{hir::BinaryOp, lex::ComparisonToken};
impl Lowerer<'_, '_> {
    pub(super) fn local(&mut self, s: Symbol, ty: &Type) -> Result<Value, Error> {
        if let Some(slot) = self.locals.get(&s) {
            return Ok(self.b.ins().stack_addr(types::I64, *slot, 0));
        }
        let len = u32::try_from(size(ty)?).map_err(err)?;
        let a = align(ty)?;
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            len,
            a.trailing_zeros() as u8,
        ));
        self.locals.insert(s, slot);
        Ok(self.b.ins().stack_addr(types::I64, slot, 0))
    }
    fn id_addr(&mut self, s: Symbol) -> Result<Value, Error> {
        if self.variables.contains_key(&s) {
            return Err(unsupported("SSA object storage unexpectedly exposed"));
        }
        if let Some(slot) = self.locals.get(&s) {
            return Ok(self.b.ins().stack_addr(types::I64, *slot, 0));
        }
        match self
            .c
            .ids
            .get(&s)
            .copied()
            .ok_or_else(|| unsupported(format!("unknown symbol {}", s.get().id)))?
        {
            Id::Data(id) => {
                let gv = self.c.module.declare_data_in_func(id, self.b.func);
                Ok(self.b.ins().symbol_value(types::I64, gv))
            }
            Id::Function(id) => {
                let f = self.c.module.declare_func_in_func(id, self.b.func);
                Ok(self.b.ins().func_addr(types::I64, f))
            }
        }
    }
    pub(super) fn address(&mut self, e: &Expr) -> Result<Value, Error> {
        match &e.expr {
            ExprType::Id(s) => self.id_addr(*s),
            ExprType::Noop(v) => self.expr(v),
            ExprType::Cast(v) | ExprType::StaticRef(v) => self.address(v),
            ExprType::Deref(v) => self.expr(v),
            ExprType::Member(base, member) => {
                let a = self.address(base)?;
                let off = member_offset(&base.ctype, *member)?;
                Ok(self.b.ins().iadd_imm_s(a, off as i64))
            }
            ExprType::Binary(BinaryOp::Add | BinaryOp::Sub, _, _) => self.expr(e),
            ExprType::Literal(LiteralValue::Str(b)) => {
                let id = self.c.string(b)?;
                let gv = self.c.module.declare_data_in_func(id, self.b.func);
                Ok(self.b.ins().symbol_value(types::I64, gv))
            }
            _ => Err(unsupported(format!(
                "nonaddressable expression {:?}",
                e.expr
            ))),
        }
    }
    fn float_to_integer(&mut self, value: Value, to: ir::Type, signed: bool) -> Value {
        // x64 scalar conversion instructions produce 32/64-bit integers.
        // Narrow C destinations require a representable I32 conversion first.
        let wide = if to.bits() < 32 { types::I32 } else { to };
        let converted = if signed {
            self.b.ins().fcvt_to_sint(wide, value)
        } else {
            self.b.ins().fcvt_to_uint(wide, value)
        };
        if wide == to {
            converted
        } else {
            self.b.ins().ireduce(to, converted)
        }
    }
    pub(super) fn cast(&mut self, v: Value, to: ir::Type, from: &Type) -> Value {
        let src = self.b.func.dfg.value_type(v);
        if src == to {
            return v;
        }
        if to.is_float() && src.is_float() {
            return if src.bits() < to.bits() {
                self.b.ins().fpromote(to, v)
            } else {
                self.b.ins().fdemote(to, v)
            };
        }
        if to.is_float() {
            return if signed(from) {
                self.b.ins().fcvt_from_sint(to, v)
            } else {
                self.b.ins().fcvt_from_uint(to, v)
            };
        }
        if src.is_float() {
            return self.float_to_integer(v, to, true);
        }
        if src.bits() < to.bits() {
            if signed(from) {
                self.b.ins().sextend(to, v)
            } else {
                self.b.ins().uextend(to, v)
            }
        } else {
            self.b.ins().ireduce(to, v)
        }
    }
    pub(super) fn condition(&mut self, e: &Expr) -> Result<Value, Error> {
        let v = self.expr(e)?;
        let t = self.b.func.dfg.value_type(v);
        if t.is_float() {
            let zero = if t == types::F32 {
                self.b.ins().f32const(0.0)
            } else {
                self.b.ins().f64const(0.0)
            };
            Ok(self.b.ins().fcmp(FloatCC::NotEqual, v, zero))
        } else {
            Ok(self.b.ins().icmp_imm_s(IntCC::NotEqual, v, 0))
        }
    }
    pub(super) fn expr(&mut self, e: &Expr) -> Result<Value, Error> {
        let ty = if e.ctype == Type::Void {
            types::I32
        } else {
            ir_type(&e.ctype)?
        };
        match &e.expr {
            // Typed HIR identifiers denote storage; scalar rvalue conversion
            // is represented by an explicit Deref node around that address.
            ExprType::Id(s) => self.id_addr(*s),
            ExprType::Literal(l) => Ok(match l {
                LiteralValue::Int(n) => self.b.ins().iconst(ty, *n),
                LiteralValue::UnsignedInt(n) => self.b.ins().iconst(ty, *n as i64),
                LiteralValue::Char(n) => self.b.ins().iconst(ty, *n as i64),
                LiteralValue::Float(n) => {
                    if ty == types::F32 {
                        self.b.ins().f32const(*n as f32)
                    } else {
                        self.b.ins().f64const(*n)
                    }
                }
                LiteralValue::Str(_) => return self.address(e),
            }),
            ExprType::StaticRef(v) => self.address(v),
            ExprType::Noop(v) => self.expr(v),
            ExprType::Sizeof(t) => Ok(self.b.ins().iconst(ty, size(t)? as i64)),
            ExprType::Cast(v) => {
                let val = self.expr(v)?;
                if e.ctype == Type::Void {
                    return Ok(val);
                }
                if e.ctype == Type::Bool {
                    let src = self.b.func.dfg.value_type(val);
                    let truth = if src.is_float() {
                        let z = if src == types::F32 {
                            self.b.ins().f32const(0.0)
                        } else {
                            self.b.ins().f64const(0.0)
                        };
                        self.b.ins().fcmp(FloatCC::NotEqual, val, z)
                    } else {
                        self.b.ins().icmp_imm_s(IntCC::NotEqual, val, 0)
                    };
                    return Ok(self.cast(truth, ty, &Type::Bool));
                }
                if self.b.func.dfg.value_type(val).is_float() && !ty.is_float() {
                    return Ok(self.float_to_integer(val, ty, signed(&e.ctype)));
                }
                Ok(self.cast(val, ty, &v.ctype))
            }
            ExprType::Deref(v) => {
                if let ExprType::Id(s) = v.expr {
                    if let Some(var) = self.variables.get(&s).copied() {
                        return Ok(self.b.use_var(var));
                    }
                }
                let a = self.expr(v)?;
                if address_type(&e.ctype) {
                    Ok(a)
                } else {
                    Ok(self.b.ins().load(ty, MemFlagsData::new(), a, 0))
                }
            }
            ExprType::Member(..) => self.address(e),
            ExprType::Negate(v) => {
                let x = self.expr(v)?;
                let x = self.cast(x, ty, &v.ctype);
                Ok(if ty.is_float() {
                    self.b.ins().fneg(x)
                } else {
                    self.b.ins().ineg(x)
                })
            }
            ExprType::BitwiseNot(v) => {
                let x = self.expr(v)?;
                let x = self.cast(x, ty, &v.ctype);
                Ok(self.b.ins().bnot(x))
            }
            ExprType::Comma(a, b) => {
                self.expr(a)?;
                self.expr(b)
            }
            ExprType::PostIncrement(v, increment) => {
                let var = if let ExprType::Id(s) = v.expr {
                    self.variables.get(&s).copied()
                } else {
                    None
                };
                let a = if var.is_none() {
                    Some(self.address(v)?)
                } else {
                    None
                };
                let storage = ir_type(&v.ctype)?;
                let old = if let Some(var) = var {
                    self.b.use_var(var)
                } else {
                    self.b
                        .ins()
                        .load(storage, MemFlagsData::new(), a.unwrap(), 0)
                };
                let step = if let Type::Pointer(p, _) = &v.ctype {
                    size(p)? as i64
                } else {
                    1
                };
                let step = if *increment { step } else { -step };
                let new = if storage.is_float() {
                    let delta = if storage == types::F32 {
                        self.b.ins().f32const(step as f32)
                    } else {
                        self.b.ins().f64const(step as f64)
                    };
                    self.b.ins().fadd(old, delta)
                } else {
                    self.b.ins().iadd_imm_s(old, step)
                };
                let new = if v.ctype == Type::Bool {
                    self.b
                        .ins()
                        .icmp_imm_u(ir::condcodes::IntCC::NotEqual, new, 0)
                } else {
                    new
                };
                if let Some(var) = var {
                    self.b.def_var(var, new);
                } else {
                    self.b.ins().store(MemFlagsData::new(), new, a.unwrap(), 0);
                }
                Ok(self.cast(old, ty, &v.ctype))
            }
            ExprType::Ternary(cond, yes, no) => {
                let cond = self.condition(cond)?;
                let y = self.b.create_block();
                let n = self.b.create_block();
                let end = self.b.create_block();
                self.b.append_block_param(end, ty);
                self.b.ins().brif(cond, y, &[], n, &[]);
                self.b.switch_to_block(y);
                let v = self.expr(yes)?;
                let v = self.cast(v, ty, &yes.ctype);
                self.b.ins().jump(end, &[v.into()]);
                self.b.switch_to_block(n);
                let v = self.expr(no)?;
                let v = self.cast(v, ty, &no.ctype);
                self.b.ins().jump(end, &[v.into()]);
                self.b.switch_to_block(end);
                Ok(self.b.block_params(end)[0])
            }
            ExprType::Binary(op, left, right) => {
                if *op == BinaryOp::Assign {
                    if let ExprType::Id(s) = left.expr {
                        if let Some(var) = self.variables.get(&s).copied() {
                            let v = self.expr(right)?;
                            let v = self.cast(v, ir_type(&left.ctype)?, &right.ctype);
                            self.b.def_var(var, v);
                            return Ok(v);
                        }
                    }
                    let a = self.address(left)?;
                    let v = self.expr(right)?;
                    if matches!(left.ctype, Type::Struct(_) | Type::Union(_)) {
                        let count = self.b.ins().iconst(types::I64, size(&left.ctype)? as i64);
                        self.b
                            .call_memmove(self.c.module.target_config(), a, v, count);
                        return Ok(a);
                    }
                    if address_type(&left.ctype) {
                        return Err(unsupported("nonassignable array/function"));
                    }
                    let v = self.cast(v, ir_type(&left.ctype)?, &right.ctype);
                    self.b.ins().store(MemFlagsData::new(), v, a, 0);
                    return Ok(v);
                }
                if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    let first = self.condition(left)?;
                    let rhs = self.b.create_block();
                    let short = self.b.create_block();
                    let end = self.b.create_block();
                    self.b.append_block_param(end, ty);
                    if *op == BinaryOp::LogicalAnd {
                        self.b.ins().brif(first, rhs, &[], short, &[]);
                    } else {
                        self.b.ins().brif(first, short, &[], rhs, &[]);
                    }
                    self.b.switch_to_block(short);
                    let v = self
                        .b
                        .ins()
                        .iconst(ty, i64::from(*op == BinaryOp::LogicalOr));
                    self.b.ins().jump(end, &[v.into()]);
                    self.b.switch_to_block(rhs);
                    let v = self.condition(right)?;
                    let v = self.cast(v, ty, &Type::Bool);
                    self.b.ins().jump(end, &[v.into()]);
                    self.b.switch_to_block(end);
                    return Ok(self.b.block_params(end)[0]);
                }
                let l = self.expr(left)?;
                let r = self.expr(right)?;
                if *op == BinaryOp::Sub {
                    if let (Type::Pointer(p, _), Type::Pointer(..)) = (&left.ctype, &right.ctype) {
                        let delta = self.b.ins().isub(l, r);
                        let scale = self.b.ins().iconst(types::I64, size(p)? as i64);
                        return Ok(self.b.ins().sdiv(delta, scale));
                    }
                }
                if matches!(op, BinaryOp::Add | BinaryOp::Sub) {
                    // HIR already scales the index. Indexed lvalues may retain
                    // their element C type while representing a byte address.
                    let pointer_left = matches!(left.ctype, Type::Pointer(..));
                    let pointer_right = matches!(right.ctype, Type::Pointer(..));
                    if pointer_left || pointer_right {
                        let l = self.cast(l, types::I64, &left.ctype);
                        let r = self.cast(r, types::I64, &right.ctype);
                        return Ok(if *op == BinaryOp::Sub {
                            self.b.ins().isub(l, r)
                        } else {
                            self.b.ins().iadd(l, r)
                        });
                    }
                }
                let lt = self.b.func.dfg.value_type(l);
                let rt = self.b.func.dfg.value_type(r);
                let operation = if matches!(op, BinaryOp::Compare(_)) {
                    if lt.is_float() || rt.is_float() {
                        if lt == types::F64 || rt == types::F64 {
                            types::F64
                        } else {
                            types::F32
                        }
                    } else if lt.bits() >= rt.bits() {
                        lt
                    } else {
                        rt
                    }
                } else {
                    ty
                };
                let l = self.cast(l, operation, &left.ctype);
                let r = self.cast(r, operation, &right.ctype);
                if let BinaryOp::Compare(cmp) = op {
                    let v = if operation.is_float() {
                        let cc = match cmp {
                            ComparisonToken::Less => FloatCC::LessThan,
                            ComparisonToken::Greater => FloatCC::GreaterThan,
                            ComparisonToken::EqualEqual => FloatCC::Equal,
                            ComparisonToken::NotEqual => FloatCC::NotEqual,
                            ComparisonToken::LessEqual => FloatCC::LessThanOrEqual,
                            ComparisonToken::GreaterEqual => FloatCC::GreaterThanOrEqual,
                        };
                        self.b.ins().fcmp(cc, l, r)
                    } else {
                        let s = signed(&left.ctype);
                        let cc = match cmp {
                            ComparisonToken::Less => {
                                if s {
                                    IntCC::SignedLessThan
                                } else {
                                    IntCC::UnsignedLessThan
                                }
                            }
                            ComparisonToken::Greater => {
                                if s {
                                    IntCC::SignedGreaterThan
                                } else {
                                    IntCC::UnsignedGreaterThan
                                }
                            }
                            ComparisonToken::EqualEqual => IntCC::Equal,
                            ComparisonToken::NotEqual => IntCC::NotEqual,
                            ComparisonToken::LessEqual => {
                                if s {
                                    IntCC::SignedLessThanOrEqual
                                } else {
                                    IntCC::UnsignedLessThanOrEqual
                                }
                            }
                            ComparisonToken::GreaterEqual => {
                                if s {
                                    IntCC::SignedGreaterThanOrEqual
                                } else {
                                    IntCC::UnsignedGreaterThanOrEqual
                                }
                            }
                        };
                        self.b.ins().icmp(cc, l, r)
                    };
                    return Ok(self.cast(v, ty, &Type::Bool));
                }
                Ok(if operation.is_float() {
                    match op {
                        BinaryOp::Add => self.b.ins().fadd(l, r),
                        BinaryOp::Sub => self.b.ins().fsub(l, r),
                        BinaryOp::Mul => self.b.ins().fmul(l, r),
                        BinaryOp::Div => self.b.ins().fdiv(l, r),
                        _ => return Err(unsupported("floating operator")),
                    }
                } else {
                    match op {
                        BinaryOp::Add => self.b.ins().iadd(l, r),
                        BinaryOp::Sub => self.b.ins().isub(l, r),
                        BinaryOp::Mul => self.b.ins().imul(l, r),
                        BinaryOp::Div => {
                            if signed(&left.ctype) {
                                self.b.ins().sdiv(l, r)
                            } else {
                                self.b.ins().udiv(l, r)
                            }
                        }
                        BinaryOp::Mod => {
                            if signed(&left.ctype) {
                                self.b.ins().srem(l, r)
                            } else {
                                self.b.ins().urem(l, r)
                            }
                        }
                        BinaryOp::Shl => self.b.ins().ishl(l, r),
                        BinaryOp::Shr => {
                            if signed(&left.ctype) {
                                self.b.ins().sshr(l, r)
                            } else {
                                self.b.ins().ushr(l, r)
                            }
                        }
                        BinaryOp::BitwiseAnd => self.b.ins().band(l, r),
                        BinaryOp::BitwiseOr => self.b.ins().bor(l, r),
                        BinaryOp::Xor => self.b.ins().bxor(l, r),
                        _ => return Err(unsupported("binary operator")),
                    }
                })
            }
            ExprType::FuncCall(callee, args) => {
                if let Some(value) = self.stdarg_call(callee, args)? {
                    return Ok(value);
                }
                let ft = match &callee.ctype {
                    Type::Function(f) => f,
                    Type::Pointer(t, _) => {
                        if let Type::Function(f) = t.as_ref() {
                            f
                        } else {
                            return Err(unsupported("call target"));
                        }
                    }
                    _ => return Err(unsupported("call target")),
                };
                if (!ft.varargs && args.len() != params(ft).len())
                    || (ft.varargs && args.len() < params(ft).len())
                {
                    return Err(unsupported("call argument count"));
                }
                let (sig, passing, result, count) = if ft.varargs {
                    let call = variadic::call_signature(ft, args)?;
                    (
                        call.signature,
                        call.arguments,
                        call.result,
                        Some(call.vector_count),
                    )
                } else {
                    let plan = abi::plan(ft, &[], true)?;
                    (plan.signature, plan.arguments, plan.result, None)
                };
                let destination = if matches!(result, abi::ResultKind::Memory) {
                    let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        u32::try_from(size(&ft.return_type)?).map_err(err)?,
                        align(&ft.return_type)?.trailing_zeros() as u8,
                    ));
                    Some(self.b.ins().stack_addr(types::I64, slot, 0))
                } else {
                    None
                };
                let mut values = Vec::new();
                if let Some(a) = destination {
                    values.push(a);
                }
                for (arg, p) in args.iter().zip(&passing) {
                    values.extend(self.argument_values(arg, p)?);
                }
                if let Some(count) = count {
                    values.push(self.b.ins().iconst(types::I32, i64::from(count)));
                }
                let direct = if !ft.varargs {
                    if let ExprType::Id(s) = &callee.expr {
                        self.c.ids.get(s).copied()
                    } else {
                        None
                    }
                } else {
                    None
                };
                let call = if let Some(Id::Function(id)) = direct {
                    let f = self.c.module.declare_func_in_func(id, self.b.func);
                    self.b.ins().call(f, &values)
                } else {
                    let address = self.expr(callee)?;
                    let sig = self.b.import_signature(sig);
                    self.b.ins().call_indirect(sig, address, &values)
                };
                match result {
                    abi::ResultKind::Void => Ok(self.b.ins().iconst(types::I32, 0)),
                    abi::ResultKind::Memory => Ok(destination.unwrap()),
                    abi::ResultKind::Scalar(_) => Ok(self.b.inst_results(call)[0]),
                    abi::ResultKind::Registers(_) => {
                        let values = self.b.inst_results(call).to_vec();
                        self.aggregate_from_registers(&ft.return_type, &values)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floating_conversions_to_narrow_integers_do_not_emit_unsupported_x64_widths() {
        let bytes = compile(
            include_str!("../tools/amd64/float-narrow.c"),
            Opt::default(),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
}
