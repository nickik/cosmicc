//! MEMORY transport and by-value snapshots; abi.rs classifies register aggregates.
use super::*;
pub(super) fn supported_layout(ty: &Type) -> bool {
    match ty {
        Type::Struct(s) | Type::Union(s) => {
            !s.is_empty() && s.members().iter().all(|m| supported_layout(&m.ctype))
        }
        Type::Array(element, ArrayType::Fixed(_)) => supported_layout(element),
        Type::Bool
        | Type::Char(_)
        | Type::SignedChar
        | Type::Short(_)
        | Type::Int(_)
        | Type::Long(_)
        | Type::LongLong(_)
        | Type::Enum(..)
        | Type::Pointer(..)
        | Type::Float
        | Type::Double => true,
        _ => false,
    }
}
pub(super) fn parameter(ty: &Type) -> Result<AbiParam, Error> {
    let length = size(ty)?;
    if length <= 16 || align(ty)? > 8 || !supported_layout(ty) {
        return Err(unsupported("aggregate by-value ABI requires supported MEMORY class (>16 bytes, natural scalar layout)"));
    }
    let padded = length
        .checked_add(7)
        .map(|n| n & !7)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| unsupported("aggregate ABI size overflow"))?;
    Ok(AbiParam::special(
        types::I64,
        ir::ArgumentPurpose::StructArgument(padded),
    ))
}
impl Lowerer<'_, '_> {
    pub(super) fn call_argument(
        &mut self,
        expr: &Expr,
        parameter: &AbiParam,
    ) -> Result<Value, Error> {
        let value = self.expr(expr)?;
        if let ir::ArgumentPurpose::StructArgument(padded) = parameter.purpose {
            // Snapshot aggregate rvalues during argument evaluation and reserve
            // padding so the backend's rounded stack memcpy cannot overread.
            let slot = self.b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                padded,
                3,
            ));
            let address = self.b.ins().stack_addr(types::I64, slot, 0);
            let length = size(&expr.ctype)?;
            if length > u64::from(padded) {
                return Err(unsupported("aggregate argument layout mismatch"));
            }
            if length != u64::from(padded) {
                let zero = self.b.ins().iconst(types::I8, 0);
                let bytes = self.b.ins().iconst(types::I64, i64::from(padded));
                self.b
                    .call_memset(self.c.module.target_config(), address, zero, bytes);
            }
            let bytes = self.b.ins().iconst(types::I64, length as i64);
            self.b
                .call_memmove(self.c.module.target_config(), address, value, bytes);
            Ok(address)
        } else {
            Ok(self.cast(value, parameter.value_type, &expr.ctype))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_and_variadic_memory_parameters_emit_elf() {
        let source="struct M{long v[4];}; long f(struct M m,int n,...){m.v[1]=99;return m.v[0]+n;} int main(void){struct M m={{7,8,9,10}};return (int)f(m,3,m);}";
        let bytes = compile(source, Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn memory_return_definitions_emit_elf() {
        let bytes = compile(
            include_str!("../tools/amd64/aggregate-return-cosmic.c"),
            Opt::default(),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn register_aggregate_parameters_and_returns_emit_elf() {
        let bytes = compile(
            include_str!("../tools/amd64/aggregate-register-cosmic.c"),
            Opt::default(),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
}
