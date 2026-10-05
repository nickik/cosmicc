//! Cosmic SIA variadic ABI: final hidden pointer to a caller-owned snapshot
//! of promoted unnamed arguments, each object rounded to four-byte alignment.
use super::*;
fn symbol(e: &Expr) -> Option<Symbol> {
    match &e.expr {
        ExprType::Id(s) => Some(*s),
        ExprType::Cast(x) | ExprType::Noop(x) | ExprType::StaticRef(x) => symbol(x),
        _ => None,
    }
}
pub(super) fn packed_size(ty: &Type, loc: Location) -> Result<u32, Error> {
    let n = ty.sizeof().map_err(|e| unsupported(loc, e.to_string()))?;
    if n == 0 {
        return Err(unsupported(
            loc,
            "variadic argument requires complete object type",
        ));
    }
    u32::try_from(
        n.checked_add(3)
            .ok_or_else(|| unsupported(loc, "variadic object too large"))?
            & !3,
    )
    .map_err(|_| unsupported(loc, "variadic object too large"))
}
impl FunctionLowerer<'_, '_, '_> {
    pub(super) fn compile_stdarg_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        loc: Location,
    ) -> Result<Option<Value>, Error> {
        let Some(s) = symbol(callee) else {
            return Ok(None);
        };
        let name = s.get().id.resolve_and_clone();
        if !name.starts_with("__cosmic_sia_va_") {
            return Ok(None);
        }
        let count = if name == "__cosmic_sia_va_end" { 1 } else { 2 };
        if args.len() != count {
            return Err(unsupported(loc, "SIA stdarg builtin argument count"));
        }
        if name == "__cosmic_sia_va_start" {
            let (pack, last) = self
                .variadic_pack
                .ok_or_else(|| unsupported(loc, "va_start requires a variadic definition"))?;
            if symbol(&args[1]) != last {
                return Err(unsupported(loc, "va_start requires last named parameter"));
            }
            let ap = self.compile_expr(&args[0])?;
            self.builder.ins().store(MemFlagsData::new(), pack, ap, 0);
        } else if name == "__cosmic_sia_va_copy" {
            let dst = self.compile_expr(&args[0])?;
            let src = self.compile_expr(&args[1])?;
            let value = self
                .builder
                .ins()
                .load(types::I32, MemFlagsData::new(), src, 0);
            self.builder.ins().store(MemFlagsData::new(), value, dst, 0);
        } else if name == "__cosmic_sia_va_end" {
            self.compile_expr(&args[0])?;
        } else if name == "__cosmic_sia_va_arg" {
            let size = args[1].clone().const_fold().map_err(source_error)?;
            let n = match size.expr {
                ExprType::Literal(LiteralValue::UnsignedInt(n)) => n,
                ExprType::Literal(LiteralValue::Int(n)) if n > 0 => n as u64,
                _ => {
                    return Err(unsupported(
                        loc,
                        "va_arg requires a complete constant-sized type",
                    ))
                }
            };
            let bytes = n
                .checked_add(3)
                .filter(|_| n > 0)
                .and_then(|v| u32::try_from(v & !3).ok())
                .ok_or_else(|| unsupported(loc, "va_arg type is incomplete or too large"))?;
            let ap = self.compile_expr(&args[0])?;
            let cursor = self
                .builder
                .ins()
                .load(types::I32, MemFlagsData::new(), ap, 0);
            let next = self.builder.ins().iadd_imm_s(cursor, i64::from(bytes));
            self.builder.ins().store(MemFlagsData::new(), next, ap, 0);
            return Ok(Some(cursor));
        } else {
            return Err(unsupported(loc, "unknown SIA stdarg builtin"));
        }
        Ok(Some(self.builder.ins().iconst(types::I32, 0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_and_variadic_c_fixture_compiles() {
        let artifact = compile_default(include_str!("../tools/sia/aggregate-varargs.c")).unwrap();
        assert!(artifact.function("main").is_some());
    }
    #[test]
    fn nonvariadic_va_start_is_rejected() {
        let error = compile_default(
            "#include <stdarg.h>\nint f(int n){va_list ap;va_start(ap,n);return 0;}",
        )
        .unwrap_err();
        assert!(format!("{error}").contains("variadic definition"));
    }
    #[test]
    fn wrong_named_va_start_parameter_is_rejected() {
        let error = compile_default("#include <stdarg.h>\nint f(int first,int last,...){va_list ap;va_start(ap,first);return 0;}").unwrap_err();
        assert!(format!("{error}").contains("last named parameter"));
    }
}
