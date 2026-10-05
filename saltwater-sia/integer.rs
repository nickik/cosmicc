//! Integer-only legalization for C I64 arithmetic, using the normal SIA backend.
use super::FunctionLowerer;
use cranelift_codegen::ir::{condcodes::IntCC, types, InstBuilder, TrapCode, Value};
impl FunctionLowerer<'_, '_, '_> {
    fn choose_word(&mut self, condition: Value, yes: Value, no: Value) -> Value {
        let bit = self.builder.ins().uextend(types::I32, condition);
        let zero = self.builder.ins().iconst(types::I32, 0);
        let mask = self.builder.ins().isub(zero, bit);
        let difference = self.builder.ins().bxor(yes, no);
        let selected = self.builder.ins().band(difference, mask);
        self.builder.ins().bxor(no, selected)
    }
    fn words64(&mut self, value: Value) -> (Value, Value) {
        let low = self.builder.ins().ireduce(types::I32, value);
        let count = self.builder.ins().iconst(types::I64, 32);
        let high = self.builder.ins().ushr(value, count);
        let high = self.builder.ins().ireduce(types::I32, high);
        (low, high)
    }
    fn join64(&mut self, low: Value, high: Value) -> Value {
        let low = self.builder.ins().uextend(types::I64, low);
        let high = self.builder.ins().uextend(types::I64, high);
        let count = self.builder.ins().iconst(types::I64, 32);
        let high = self.builder.ins().ishl(high, count);
        self.builder.ins().bor(low, high)
    }
    pub(super) fn multiply64(&mut self, a: Value, b: Value) -> Value {
        let (al, ah) = self.words64(a);
        let (bl, bh) = self.words64(b);
        let mask = self.builder.ins().iconst(types::I32, 65535);
        let count = self.builder.ins().iconst(types::I32, 16);
        let a0 = self.builder.ins().band(al, mask);
        let a1 = self.builder.ins().ushr(al, count);
        let b0 = self.builder.ins().band(bl, mask);
        let b1 = self.builder.ins().ushr(bl, count);
        let mut result = self.builder.ins().iconst(types::I64, 0);
        for (x, y, shift) in [
            (a0, b0, 0),
            (a0, b1, 16),
            (a1, b0, 16),
            (a1, b1, 32),
            (ah, bl, 32),
            (al, bh, 32),
        ] {
            let product = self.builder.ins().imul(x, y);
            let product = self.builder.ins().uextend(types::I64, product);
            let count = self.builder.ins().iconst(types::I64, shift);
            let product = self.builder.ins().ishl(product, count);
            result = self.builder.ins().iadd(result, product);
        }
        result
    }
    pub(super) fn shift64(
        &mut self,
        value: Value,
        count: Value,
        signed_right: bool,
        left: bool,
    ) -> Value {
        let (lo, hi) = self.words64(value);
        let count = self.builder.ins().ireduce(types::I32, count);
        let mask = self.builder.ins().iconst(types::I32, 63);
        let count = self.builder.ins().band(count, mask);
        let zero = self.builder.ins().iconst(types::I32, 0);
        let width = self.builder.ins().iconst(types::I32, 32);
        let large = self
            .builder
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, count, width);
        let is_zero = self.builder.ins().icmp(IntCC::Equal, count, zero);
        let back = self.builder.ins().isub(width, count);
        let extra = self.builder.ins().isub(count, width);
        let (small_lo, small_hi, large_lo, large_hi) = if left {
            let low = self.builder.ins().ishl(lo, count);
            let high = self.builder.ins().ishl(hi, count);
            let carry = self.builder.ins().ushr(lo, back);
            let carry = self.choose_word(is_zero, zero, carry);
            let high = self.builder.ins().bor(high, carry);
            let large_high = self.builder.ins().ishl(lo, extra);
            (low, high, zero, large_high)
        } else {
            let low = self.builder.ins().ushr(lo, count);
            let carry = self.builder.ins().ishl(hi, back);
            let carry = self.choose_word(is_zero, zero, carry);
            let low = self.builder.ins().bor(low, carry);
            let high = if signed_right {
                self.builder.ins().sshr(hi, count)
            } else {
                self.builder.ins().ushr(hi, count)
            };
            let large_low = if signed_right {
                self.builder.ins().sshr(hi, extra)
            } else {
                self.builder.ins().ushr(hi, extra)
            };
            let sign_count = self.builder.ins().iconst(types::I32, 31);
            let large_high = if signed_right {
                self.builder.ins().sshr(hi, sign_count)
            } else {
                zero
            };
            (low, high, large_low, large_high)
        };
        let low = self.choose_word(large, large_lo, small_lo);
        let high = self.choose_word(large, large_hi, small_hi);
        self.join64(low, high)
    }
    pub(super) fn remainder64(
        &mut self,
        numerator: Value,
        denominator: Value,
        signed: bool,
    ) -> Value {
        self.divrem64(numerator, denominator, signed, false)
    }
    pub(super) fn division64(
        &mut self,
        numerator: Value,
        denominator: Value,
        signed: bool,
    ) -> Value {
        self.divrem64(numerator, denominator, signed, true)
    }
    fn divrem64(
        &mut self,
        numerator: Value,
        denominator: Value,
        signed: bool,
        quotient: bool,
    ) -> Value {
        let zero = self.builder.ins().iconst(types::I64, 0);
        let nonzero = self.builder.ins().icmp(IntCC::NotEqual, denominator, zero);
        self.builder
            .ins()
            .trapz(nonzero, TrapCode::INTEGER_DIVISION_BY_ZERO);
        let shift63 = self.builder.ins().iconst(types::I64, 63);
        let sign = if signed {
            let min = self.builder.ins().iconst(types::I64, i64::MIN);
            let minus1 = self.builder.ins().iconst(types::I64, -1);
            let is_min = self.builder.ins().icmp(IntCC::Equal, numerator, min);
            let is_minus1 = self.builder.ins().icmp(IntCC::Equal, denominator, minus1);
            let overflow = self.builder.ins().band(is_min, is_minus1);
            self.builder
                .ins()
                .trapnz(overflow, TrapCode::INTEGER_OVERFLOW);
            self.builder.ins().sshr(numerator, shift63)
        } else {
            zero
        };
        let n_xor = self.builder.ins().bxor(numerator, sign);
        let n = self.builder.ins().isub(n_xor, sign);
        let d_sign = if signed {
            self.builder.ins().sshr(denominator, shift63)
        } else {
            zero
        };
        let result_sign = if quotient {
            self.builder.ins().bxor(sign, d_sign)
        } else {
            sign
        };
        let d = if signed {
            let d_xor = self.builder.ins().bxor(denominator, d_sign);
            self.builder.ins().isub(d_xor, d_sign)
        } else {
            denominator
        };
        let loop_block = self.builder.create_block();
        let subtract = self.builder.create_block();
        let advance = self.builder.create_block();
        let done = self.builder.create_block();
        for ty in [types::I64, types::I64, types::I32, types::I64] {
            self.builder.append_block_param(loop_block, ty);
        }
        self.builder.append_block_param(advance, types::I64);
        self.builder.append_block_param(advance, types::I64);
        self.builder.append_block_param(done, types::I64);
        let count = self.builder.ins().iconst(types::I32, 64);
        self.builder.ins().jump(
            loop_block,
            &[n.into(), zero.into(), count.into(), zero.into()],
        );
        self.builder.switch_to_block(loop_block);
        let params = self.builder.block_params(loop_block).to_vec();
        let n = params[0];
        let r = params[1];
        let count = params[2];
        let q = params[3];
        let carry = self.builder.ins().ushr(r, shift63);
        let carry = self.builder.ins().icmp(IntCC::NotEqual, carry, zero);
        let bit = self.builder.ins().ushr(n, shift63);
        let one = self.builder.ins().iconst(types::I64, 1);
        let r_shift = self.builder.ins().ishl(r, one);
        let shifted = self.builder.ins().bor(r_shift, bit);
        let n_next = self.builder.ins().ishl(n, one);
        let q_next = self.builder.ins().ishl(q, one);
        let ge = self
            .builder
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, shifted, d);
        let needs_subtract = self.builder.ins().bor(carry, ge);
        self.builder.ins().brif(
            needs_subtract,
            subtract,
            &[],
            advance,
            &[shifted.into(), q_next.into()],
        );
        self.builder.switch_to_block(subtract);
        let difference = self.builder.ins().isub(shifted, d);
        let q_set = self.builder.ins().bor(q_next, one);
        self.builder
            .ins()
            .jump(advance, &[difference.into(), q_set.into()]);
        self.builder.switch_to_block(advance);
        let remainder = self.builder.block_params(advance)[0];
        let q = self.builder.block_params(advance)[1];
        let one32 = self.builder.ins().iconst(types::I32, 1);
        let next = self.builder.ins().isub(count, one32);
        let zero32 = self.builder.ins().iconst(types::I32, 0);
        let again = self.builder.ins().icmp(IntCC::NotEqual, next, zero32);
        self.builder.ins().brif(
            again,
            loop_block,
            &[n_next.into(), remainder.into(), next.into(), q.into()],
            done,
            &[if quotient { q.into() } else { remainder.into() }],
        );
        self.builder.switch_to_block(done);
        let unsigned = self.builder.block_params(done)[0];
        let xor = self.builder.ins().bxor(unsigned, result_sign);
        self.builder.ins().isub(xor, result_sign)
    }
}
