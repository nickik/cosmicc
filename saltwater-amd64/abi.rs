//! System V scalar and naturally laid out INTEGER/SSE aggregate classification.
use super::*;
#[derive(Clone)]
pub(super) enum Passing {
    Scalar(AbiParam),
    Memory(u32),
    Registers(Vec<ir::Type>),
}
#[derive(Clone)]
pub(super) enum ResultKind {
    Void,
    Scalar(AbiParam),
    Memory,
    Registers(Vec<ir::Type>),
}
pub(super) struct Plan {
    pub signature: Signature,
    pub arguments: Vec<Passing>,
    pub result: ResultKind,
    pub gp: u32,
    pub fp: u32,
    pub stack: u32,
}
fn aggregate_type(t: &Type) -> bool {
    matches!(t, Type::Struct(_) | Type::Union(_))
}
pub(super) fn classes(ty: &Type) -> Result<Vec<ir::Type>, Error> {
    if !aggregate_type(ty) || !aggregate::supported_layout(ty) || align(ty)? > 8 || size(ty)? > 16 {
        return Err(unsupported("aggregate register classification requires a natural scalar layout of at most 16 bytes"));
    }
    let mut out = vec![None; ((size(ty)? + 7) / 8) as usize];
    fn walk(t: &Type, base: u64, out: &mut [Option<ir::Type>]) -> Result<(), Error> {
        match t {
            Type::Struct(s) | Type::Union(s) => {
                for m in s.members().iter() {
                    let off = if matches!(t, Type::Union(_)) {
                        0
                    } else {
                        s.offset_for(m.id, TargetDataModel::Amd64)
                    };
                    walk(&m.ctype, base + off, out)?;
                }
            }
            Type::Array(t, ArrayType::Fixed(n)) => {
                for i in 0..*n {
                    walk(t, base + i * size(t)?, out)?;
                }
            }
            _ => {
                let class = if ir_type(t)?.is_float() {
                    types::F64
                } else {
                    types::I64
                };
                for i in base / 8..=(base + size(t)? - 1) / 8 {
                    let slot = &mut out[i as usize];
                    *slot = Some(if *slot == Some(types::I64) || class == types::I64 {
                        types::I64
                    } else {
                        types::F64
                    });
                }
            }
        }
        Ok(())
    }
    walk(ty, 0, &mut out)?;
    Ok(out.into_iter().map(|v| v.unwrap_or(types::I64)).collect())
}
fn rounded(ty: &Type) -> Result<u32, Error> {
    u32::try_from((size(ty)? + 7) & !7).map_err(err)
}
pub(super) fn plan(ft: &FunctionType, extras: &[Expr], checked: bool) -> Result<Plan, Error> {
    let mut signature = Signature::new(isa::CallConv::SystemV);
    let result = if *ft.return_type == Type::Void {
        ResultKind::Void
    } else if aggregate_type(&ft.return_type) {
        if size(&ft.return_type).or_else(|e| if checked { Err(e) } else { Ok(0) })? > 16 {
            match aggregate::parameter(&ft.return_type) {
                Ok(_) => ResultKind::Memory,
                Err(e) if checked => return Err(e),
                Err(_) => ResultKind::Scalar(AbiParam::new(types::I64)),
            }
        } else {
            match classes(&ft.return_type) {
                Ok(c) => ResultKind::Registers(c),
                Err(e) if checked => return Err(e),
                Err(_) => ResultKind::Scalar(AbiParam::new(types::I64)),
            }
        }
    } else {
        ResultKind::Scalar(abi(&ft.return_type)?)
    };
    let (mut gp, mut fp, mut stack) = (0, 0, 0);
    match &result {
        ResultKind::Memory => {
            signature.params.push(AbiParam::new(types::I64));
            signature.returns.push(AbiParam::new(types::I64));
            gp = 1;
        }
        ResultKind::Scalar(p) => signature.returns.push(*p),
        ResultKind::Registers(c) => signature
            .returns
            .extend(c.iter().map(|t| AbiParam::new(*t))),
        ResultKind::Void => {}
    }
    let mut arguments = Vec::new();
    for ty in params(ft)
        .iter()
        .map(|s| s.get().ctype.clone())
        .chain(extras.iter().map(|e| e.ctype.clone()))
    {
        let passing = if aggregate_type(&ty) {
            if size(&ty).or_else(|e| if checked { Err(e) } else { Ok(0) })? > 16 {
                match aggregate::parameter(&ty) {
                    Ok(_) => Passing::Memory(rounded(&ty)?),
                    Err(e) if checked => return Err(e),
                    Err(_) => Passing::Scalar(AbiParam::new(types::I64)),
                }
            } else {
                match classes(&ty) {
                    Ok(c) => {
                        let g = c.iter().filter(|t| t.is_int()).count() as u32;
                        let f = c.len() as u32 - g;
                        if gp + g <= 6 && fp + f <= 8 {
                            gp += g;
                            fp += f;
                            Passing::Registers(c)
                        } else {
                            Passing::Memory(rounded(&ty)?)
                        }
                    }
                    Err(e) if checked => return Err(e),
                    Err(_) => Passing::Scalar(AbiParam::new(types::I64)),
                }
            }
        } else {
            Passing::Scalar(abi(&ty)?)
        };
        match &passing {
            Passing::Registers(c) => signature.params.extend(c.iter().map(|t| AbiParam::new(*t))),
            Passing::Memory(n) => {
                signature.params.push(AbiParam::special(
                    types::I64,
                    ir::ArgumentPurpose::StructArgument(*n),
                ));
                stack += n;
            }
            Passing::Scalar(p) => {
                signature.params.push(*p);
                if p.value_type.is_float() {
                    if fp < 8 {
                        fp += 1;
                    } else {
                        stack += 8;
                    }
                } else if gp < 6 {
                    gp += 1;
                } else {
                    stack += 8;
                }
            }
        }
        arguments.push(passing);
    }
    Ok(Plan {
        signature,
        arguments,
        result,
        gp,
        fp,
        stack,
    })
}
impl Lowerer<'_, '_> {
    pub(super) fn aggregate_snapshot(&mut self, source: Value, ty: &Type) -> Result<Value, Error> {
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            rounded(ty)?,
            3,
        ));
        let address = self.b.ins().stack_addr(types::I64, slot, 0);
        let zero = self.b.ins().iconst(types::I8, 0);
        let padded = self.b.ins().iconst(types::I64, i64::from(rounded(ty)?));
        self.b
            .call_memset(self.c.module.target_config(), address, zero, padded);
        let bytes = self.b.ins().iconst(types::I64, size(ty)? as i64);
        self.b
            .call_memmove(self.c.module.target_config(), address, source, bytes);
        Ok(address)
    }
    pub(super) fn argument_values(&mut self, e: &Expr, p: &Passing) -> Result<Vec<Value>, Error> {
        match p {
            Passing::Scalar(a) => Ok(vec![self.call_argument(e, a)?]),
            Passing::Memory(n) => Ok(vec![self.call_argument(
                e,
                &AbiParam::special(types::I64, ir::ArgumentPurpose::StructArgument(*n)),
            )?]),
            Passing::Registers(c) => {
                let source = self.expr(e)?;
                let address = self.aggregate_snapshot(source, &e.ctype)?;
                Ok(c.iter()
                    .enumerate()
                    .map(|(i, t)| {
                        self.b
                            .ins()
                            .load(*t, MemFlagsData::new(), address, (i * 8) as i32)
                    })
                    .collect())
            }
        }
    }
    pub(super) fn aggregate_from_registers(
        &mut self,
        ty: &Type,
        values: &[Value],
    ) -> Result<Value, Error> {
        let slot = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            rounded(ty)?,
            3,
        ));
        let address = self.b.ins().stack_addr(types::I64, slot, 0);
        for (i, v) in values.iter().enumerate() {
            self.b
                .ins()
                .store(MemFlagsData::new(), *v, address, (i * 8) as i32);
        }
        Ok(address)
    }
}
