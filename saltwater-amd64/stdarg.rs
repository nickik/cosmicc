//! Bounded AMD64 System V register-save and overflow va_list traversal.
use super::*;
use cranelift_codegen::ir::condcodes::IntCC;
pub(super) struct Incoming {
    save: Value,
    overflow: Value,
    gp: u32,
    fp: u32,
    last: Option<Symbol>,
}
fn layout(ft: &FunctionType) -> Result<(u32, u32, u32), Error> {
    let plan = abi::plan(ft, &[], true)?;
    Ok((plan.gp, plan.fp, plan.stack))
}
pub(super) fn incoming_signature(ft: &FunctionType) -> Result<Signature, Error> {
    let mut s = signature(ft, true)?;
    if ft.varargs {
        let (gp, fp, _) = layout(ft)?;
        for _ in gp..6 {
            s.params.push(AbiParam::new(types::I64));
        }
        for _ in fp..8 {
            s.params.push(AbiParam::new(types::F64));
        }
    }
    Ok(s)
}
fn symbol(e: &Expr) -> Option<Symbol> {
    match &e.expr {
        ExprType::Id(s) => Some(*s),
        ExprType::Cast(x) | ExprType::Noop(x) | ExprType::StaticRef(x) => symbol(x),
        _ => None,
    }
}
fn requested(e: &Expr) -> Option<Type> {
    if let Type::Pointer(t, _) = &e.ctype {
        if **t != Type::Void {
            return Some((**t).clone());
        }
    }
    match &e.expr {
        ExprType::Cast(x) | ExprType::Noop(x) => requested(x),
        _ => None,
    }
}
impl Lowerer<'_, '_> {
    pub(super) fn capture_variadic(
        &mut self,
        ft: &FunctionType,
        values: &[Value],
    ) -> Result<(), Error> {
        if !ft.varargs {
            return Ok(());
        }
        let (gp, fp, stack) = layout(ft)?;
        let slot =
            self.b
                .create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 176, 4));
        let save = self.b.ins().stack_addr(types::I64, slot, 0);
        let mut i = abi::plan(ft, &[], true)?.signature.params.len();
        for n in gp..6 {
            self.b
                .ins()
                .store(MemFlagsData::new(), values[i], save, (n * 8) as i32);
            i += 1;
        }
        for n in fp..8 {
            self.b
                .ins()
                .store(MemFlagsData::new(), values[i], save, (48 + n * 16) as i32);
            i += 1;
        }
        let frame = self.b.ins().get_frame_pointer(types::I64);
        let overflow = self.b.ins().iadd_imm_s(frame, i64::from(16 + stack));
        self.incoming_variadic = Some(Incoming {
            save,
            overflow,
            gp: gp * 8,
            fp: 48 + fp * 16,
            last: params(ft).last().copied(),
        });
        Ok(())
    }
    pub(super) fn stdarg_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<Value>, Error> {
        let Some(s) = symbol(callee) else {
            return Ok(None);
        };
        let name = s.get().id.resolve_and_clone();
        if !name.starts_with("__cosmic_va_") {
            return Ok(None);
        }
        let count = if name == "__cosmic_va_end" { 1 } else { 2 };
        if args.len() != count {
            return Err(unsupported("stdarg builtin argument count"));
        }
        if name == "__cosmic_va_start" {
            let state = self
                .incoming_variadic
                .as_ref()
                .ok_or_else(|| unsupported("va_start requires a variadic definition"))?;
            if symbol(&args[1]) != state.last {
                return Err(unsupported("va_start requires the last named parameter"));
            }
            let (save, overflow, gp, fp) = (state.save, state.overflow, state.gp, state.fp);
            let ap = self.expr(&args[0])?;
            let g = self.b.ins().iconst(types::I32, i64::from(gp));
            let f = self.b.ins().iconst(types::I32, i64::from(fp));
            self.b.ins().store(MemFlagsData::new(), g, ap, 0);
            self.b.ins().store(MemFlagsData::new(), f, ap, 4);
            self.b.ins().store(MemFlagsData::new(), overflow, ap, 8);
            self.b.ins().store(MemFlagsData::new(), save, ap, 16);
        } else if name == "__cosmic_va_copy" {
            let dst = self.expr(&args[0])?;
            let src = self.expr(&args[1])?;
            let bytes = self.b.ins().iconst(types::I64, 24);
            self.b
                .call_memmove(self.c.module.target_config(), dst, src, bytes);
        } else if name == "__cosmic_va_end" {
            self.expr(&args[0])?;
        } else if name == "__cosmic_va_arg" {
            let ty = requested(&args[1]).ok_or_else(|| unsupported("va_arg type marker"))?;
            if matches!(
                ty,
                Type::Bool | Type::Char(_) | Type::SignedChar | Type::Short(_) | Type::Float
            ) {
                return Err(unsupported(
                    "va_arg requires a default-promoted type (int or double)",
                ));
            }
            let ap = self.expr(&args[0])?;
            let overflow = self.b.ins().load(types::I64, MemFlagsData::new(), ap, 8);
            let plus = self.b.ins().iadd_imm_s(overflow, 7);
            let aligned = self.b.ins().band_imm_s(plus, -8);
            if matches!(ty, Type::Struct(_) | Type::Union(_)) {
                let padded = u32::try_from((size(&ty)? + 7) & !7).map_err(err)?;
                if size(&ty)? > 16 {
                    aggregate::parameter(&ty)?;
                    let next = self.b.ins().iadd_imm_s(aligned, i64::from(padded));
                    self.b.ins().store(MemFlagsData::new(), next, ap, 8);
                    return Ok(Some(aligned));
                }
                let classes = abi::classes(&ty)?;
                let gp_needed = classes.iter().filter(|t| t.is_int()).count() as u32;
                let fp_needed = classes.len() as u32 - gp_needed;
                let gp = self.b.ins().load(types::I32, MemFlagsData::new(), ap, 0);
                let fp = self.b.ins().load(types::I32, MemFlagsData::new(), ap, 4);
                let has_gp = if gp_needed == 0 {
                    self.b.ins().iconst(types::I8, 1)
                } else {
                    self.b.ins().icmp_imm_u(
                        IntCC::UnsignedLessThanOrEqual,
                        gp,
                        i64::from(48 - gp_needed * 8),
                    )
                };
                let has_fp = if fp_needed == 0 {
                    self.b.ins().iconst(types::I8, 1)
                } else {
                    self.b.ins().icmp_imm_u(
                        IntCC::UnsignedLessThanOrEqual,
                        fp,
                        i64::from(176 - fp_needed * 16),
                    )
                };
                let available = self.b.ins().band(has_gp, has_fp);
                let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    padded,
                    3,
                ));
                let address = self.b.ins().stack_addr(types::I64, slot, 0);
                let registers = self.b.create_block();
                let stack = self.b.create_block();
                let done = self.b.create_block();
                self.b.ins().brif(available, registers, &[], stack, &[]);
                self.b.switch_to_block(registers);
                let base = self.b.ins().load(types::I64, MemFlagsData::new(), ap, 16);
                let (mut g, mut f) = (gp, fp);
                for (i, t) in classes.iter().enumerate() {
                    let offset = if t.is_int() { g } else { f };
                    let wide = self.b.ins().uextend(types::I64, offset);
                    let source = self.b.ins().iadd(base, wide);
                    let value = self.b.ins().load(*t, MemFlagsData::new(), source, 0);
                    self.b
                        .ins()
                        .store(MemFlagsData::new(), value, address, (i * 8) as i32);
                    if t.is_int() {
                        g = self.b.ins().iadd_imm_s(g, 8);
                    } else {
                        f = self.b.ins().iadd_imm_s(f, 16);
                    }
                }
                self.b.ins().store(MemFlagsData::new(), g, ap, 0);
                self.b.ins().store(MemFlagsData::new(), f, ap, 4);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(stack);
                let bytes = self.b.ins().iconst(types::I64, size(&ty)? as i64);
                self.b
                    .call_memmove(self.c.module.target_config(), address, aligned, bytes);
                let next = self.b.ins().iadd_imm_s(aligned, i64::from(padded));
                self.b.ins().store(MemFlagsData::new(), next, ap, 8);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(done);
                return Ok(Some(address));
            }
            let a = abi(&ty)?;
            let (field, limit, step) = if a.value_type.is_float() {
                (4, 176, 16)
            } else {
                (0, 48, 8)
            };
            let offset = self
                .b
                .ins()
                .load(types::I32, MemFlagsData::new(), ap, field);
            let available = self
                .b
                .ins()
                .icmp_imm_u(IntCC::UnsignedLessThan, offset, limit);
            let base = self.b.ins().load(types::I64, MemFlagsData::new(), ap, 16);
            let wide = self.b.ins().uextend(types::I64, offset);
            let reg = self.b.ins().iadd(base, wide);
            let result = self.b.ins().select(available, reg, aligned);
            let bumped = self.b.ins().iadd_imm_s(offset, step);
            let offset_next = self.b.ins().select(available, bumped, offset);
            self.b
                .ins()
                .store(MemFlagsData::new(), offset_next, ap, field);
            let advanced = self.b.ins().iadd_imm_s(aligned, 8);
            let stack_next = self.b.ins().select(available, overflow, advanced);
            self.b.ins().store(MemFlagsData::new(), stack_next, ap, 8);
            return Ok(Some(result));
        } else {
            return Err(unsupported("unknown stdarg builtin"));
        }
        Ok(Some(self.b.ins().iconst(types::I32, 0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incoming_cursor_operations_emit_elf() {
        let bytes = compile(
            include_str!("../tools/amd64/stdarg-cosmic.c"),
            Opt::default(),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn register_aggregate_va_arg_emits_elf() {
        let bytes = compile("#include <stdarg.h>\nstruct S{int a,b;};int f(int n,...){va_list ap;va_start(ap,n);return va_arg(ap,struct S).a;}",Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn unpromoted_float_va_arg_is_diagnosed() {
        let error = compile("#include <stdarg.h>\nfloat f(int n,...){va_list ap;va_start(ap,n);return va_arg(ap,float);}",Opt::default()).unwrap_err();
        assert!(format!("{error}").contains("default-promoted"));
    }
    #[test]
    fn va_start_in_fixed_definition_is_diagnosed() {
        let error = compile(
            "#include <stdarg.h>\nint f(int n){va_list ap;va_start(ap,n);return 0;}",
            Opt::default(),
        )
        .unwrap_err();
        assert!(format!("{error}").contains("variadic definition"));
    }
}
