# Incoming System V stdarg acceptance

The AMD64-only header defines the standard 24-byte va_list state. Scalar GP/SSE arguments and naturally laid out aggregate arguments use separate register cursors and an overflow cursor, consistent with the [AMD64 psABI](https://gitlab.com/x86-psABIs/x86-64-ABI/-/blob/master/x86-64-ABI/low-level-sys-info.tex).

stdarg.rs captures unused argument registers through synthetic incoming CLIF parameters. Variadic definitions preserve frame pointers to obtain the incoming stack argument area; named stack parameters and a hidden aggregate-return pointer are included in initial cursor calculations. XMM argument slots are saved unconditionally without floating arithmetic. This is a deliberate bounded implementation choice; conditional AL-based saves can reduce its overhead later.

va_start checks the last named parameter and requires a variadic definition. va_arg diagnoses unpromoted narrow/float requests and unsupported ABI classes. va_copy copies all 24 bytes; va_end evaluates its operand. Fixed functions can consume va_list values created by GCC or Clang. The private typed-marker helper uses an ellipsis prototype to preserve the requested pointer type in HIR; it emits no helper call or marker evaluation.

check-stdarg.py passed eight Cosmic O0/O2 × GCC13/Clang18 O0/O2 combinations plus four host-only references. The fixture checks GP/SSE exhaustion, ten doubles/eight longs, named arguments already overflowing both register banks, independently copied cursors, host-created lists consumed by Cosmic, Cosmic-created lists consumed by host code, MEMORY aggregate arguments and scalar default promotions. Evidence: /tmp/cosmic-native-stdarg-storage-final/summary.json. The extended register-aggregate gate checks mixed-class va_arg reconstruction and rollback.

Long double/x87, vector/overaligned or packed aggregate classes, and register-save optimizations remain outside this accepted slice. Scalar/array-field aggregate classification and return details are recorded in REGISTER_AGGREGATE_STATUS.md. Native proof does not imply SIA or Lighting acceptance.

Final storage/UTF-32 source acceptance uses compiler SHA256 f56966383461ef5bc0867d5347d9369fcbc93c7b399d76242b73e54062fddcad. All eight interoperability combinations and four references passed again after those changes.
