//! Binary32/binary64 software calls: values use integer registers, never native FP IR.
use super::*;
pub(super) const HELPERS: [&str; 33] = [
    "cosmic_sf_add32",
    "cosmic_sf_sub32",
    "cosmic_sf_mul32",
    "cosmic_sf_div32",
    "cosmic_sf_eq32",
    "cosmic_sf_lt32",
    "cosmic_sf_le32",
    "cosmic_sf_to_i32",
    "cosmic_sf_to_u32",
    "cosmic_sf_from_i32",
    "cosmic_sf_from_u32",
    "cosmic_sf_add64",
    "cosmic_sf_sub64",
    "cosmic_sf_mul64",
    "cosmic_sf_div64",
    "cosmic_sf_eq64",
    "cosmic_sf_lt64",
    "cosmic_sf_le64",
    "cosmic_sf_f32_to_f64",
    "cosmic_sf_f64_to_f32",
    "cosmic_sf_i64_to_f32",
    "cosmic_sf_u64_to_f32",
    "cosmic_sf_i64_to_f64",
    "cosmic_sf_u64_to_f64",
    "cosmic_sf_f32_to_i64",
    "cosmic_sf_f32_to_u64",
    "cosmic_sf_f64_to_i64",
    "cosmic_sf_f64_to_u64",
    "cosmic_sf_i32_to_f64",
    "cosmic_sf_u32_to_f64",
    "cosmic_sf_f64_to_i32",
    "cosmic_sf_f64_to_u32",
    "cosmic_sf_current_context",
];
impl FunctionLowerer<'_, '_, '_> {
    pub(super) fn sf_call(&mut self, index: usize, args: &[Value]) -> Value {
        // Runtime expressions always fetch the task-bound context. Calls are
        // side-effecting, so SIA never folds away observable flags/rounding.
        let mut context_signature = Signature::new(CallConv::SystemV);
        context_signature.returns.push(AbiParam::new(types::I32));
        let context_signature = self.builder.import_signature(context_signature);
        let name = self
            .builder
            .func
            .declare_imported_user_function(UserExternalName::new(3, 32));
        let function = self.builder.import_function(ExtFuncData {
            name: ExternalName::user(name),
            signature: context_signature,
            colocated: false,
            patchable: false,
        });
        let current = self.builder.ins().call(function, &[]);
        let context = self.builder.func.dfg.first_result(current);
        let mut signature = Signature::new(CallConv::SystemV);
        signature.params.push(AbiParam::new(types::I32));
        for value in args {
            signature
                .params
                .push(AbiParam::new(self.builder.func.dfg.value_type(*value)));
        }
        let result_type = if matches!(index,11..=14|18|22..=27|28..=29) {
            types::I64
        } else {
            types::I32
        };
        signature.returns.push(AbiParam::new(result_type));
        let signature = self.builder.import_signature(signature);
        let external = self
            .builder
            .func
            .declare_imported_user_function(UserExternalName::new(3, index as u32));
        let function = self.builder.import_function(ExtFuncData {
            name: ExternalName::user(external),
            signature,
            colocated: false,
            patchable: false,
        });
        let mut values = vec![context];
        values.extend_from_slice(args);
        let call = self.builder.ins().call(function, &values);
        self.builder.func.dfg.first_result(call)
    }
    pub(super) fn sf_truth(&mut self, value: Value) -> Value {
        let ty = self.builder.func.dfg.value_type(value);
        let mask = self.builder.ins().iconst(
            ty,
            if ty == types::I64 {
                i64::MAX
            } else {
                0x7fffffff
            },
        );
        let magnitude = self.builder.ins().band(value, mask);
        self.builder.ins().icmp_imm_u(IntCC::NotEqual, magnitude, 0)
    }
    pub(super) fn sf_cast(
        &mut self,
        value: Value,
        from: &Type,
        to: &Type,
        location: Location,
    ) -> Result<Value, Error> {
        let from_float = matches!(from, Type::Float | Type::Double | Type::LongDouble);
        let to_float = matches!(to, Type::Float | Type::Double | Type::LongDouble);
        if from == to
            || matches!(
                (from, to),
                (Type::Double, Type::LongDouble) | (Type::LongDouble, Type::Double)
            )
        {
            return Ok(value);
        }
        if from_float && to == &Type::Bool {
            let truth = self.sf_truth(value);
            return Ok(self.coerce_integer_value(truth, types::I8, &Type::Bool));
        }
        let helper = if from_float && to_float {
            if from == &Type::Float {
                18
            } else {
                19
            }
        } else if from_float && to.is_integral() {
            match (
                matches!(from, Type::Double | Type::LongDouble),
                matches!(to, Type::LongLong(_)),
                is_signed_integer_type(to),
            ) {
                (false, false, true) => 7,
                (false, false, false) => 8,
                (false, true, true) => 24,
                (false, true, false) => 25,
                (true, true, true) => 26,
                (true, true, false) => 27,
                (true, false, true) => 30,
                (true, false, false) => 31,
            }
        } else if from.is_integral() && to_float {
            match (
                matches!(to, Type::Double | Type::LongDouble),
                matches!(from, Type::LongLong(_)),
                is_signed_integer_type(from),
            ) {
                (false, false, true) => 9,
                (false, false, false) => 10,
                (false, true, true) => 20,
                (false, true, false) => 21,
                (true, true, true) => 22,
                (true, true, false) => 23,
                (true, false, true) => 28,
                (true, false, false) => 29,
            }
        } else {
            return Err(unsupported(
                location,
                "software floating-point conversion requires an arithmetic or bool type",
            ));
        };
        let input = if from_float || matches!(from, Type::LongLong(_)) {
            value
        } else {
            self.coerce_integer_value(value, types::I32, from)
        };
        let result = self.sf_call(helper, &[input]);
        if to_float {
            Ok(result)
        } else {
            Ok(self.coerce_integer_value(result, ir_type(to, location)?, to))
        }
    }
}
