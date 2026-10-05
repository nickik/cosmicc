# Cosmic SIA aggregate and variable-argument ABI

Struct/union parameters are transported as I32 addresses of caller-created snapshots. The callee copies each object into its own storage before executing the body. Aggregate results use a leading hidden I32 destination pointer. Calls return the destination address as the HIR aggregate expression value. These conventions apply to direct and indirect calls and are private Cosmic SIA conventions, not AMD64 SystemV or a general external C ABI.

A variadic call passes the fixed parameters normally, then one final hidden I32 pointer to a caller-owned pack of its promoted unnamed arguments. The pack preserves source order and rounds each object's size up to four bytes. Promoted I64/binary64 values occupy eight bytes, with four-byte pack alignment; aggregate object representations are copied inline. There is no count or runtime type metadata: va_arg must request the actual promoted argument type, as required by C. This convention lets the backend handle fixed and hidden parameters through its established register/stack ABI, including functions whose named arguments exceed register capacity.

The SIA target stdarg.h defines va_list as an unsigned-byte pointer. va_start stores the incoming hidden pointer; va_arg returns the current typed slot and advances by the rounded size; va_copy copies the cursor; va_end evaluates its argument without machine effects. A function receiving va_list by value receives an independent pointer copy. Use va_list* when intentionally advancing the caller's cursor. va_start is diagnosed outside a variadic definition or with a different last named parameter. As in C, requesting a type different from the actual promoted argument has undefined behavior; cursor traversal does not add dynamic type checks. va_arg requires a complete constant-sized object type. Binary64 execution depends on the target software runtime linked into the guest.

The native AMD64 stdarg header and register-save implementation remain separate. No interoperability with host va_list or a hardware-defined SIA varargs convention is claimed.

LightingMachine execution passed the exact 4,800-byte CLI container in 3,810 instructions with main returning 0 and restored stack pointer. tools/sia/check-aggregate-varargs.py reproduces this proof. tools/sia/aggregate-varargs.c covers copied small/large aggregates, aggregate returns, direct/indirect calls with seven named parameters, eight unnamed integers, I64, inline aggregate va_arg and independent va_copy.

The final cross-TU gate linked six serialized objects and passed **20,706**
instructions on both LightingMachine and production CPU/MMU/module RTL with
real MainboardFPGA in Bluesim. It includes binary64 values, float default
promotion, I64 packed arguments and indirect aggregate returns. The final
focused aggregate fixture passed **3,877** production instructions after the
full-width bool fix (the earlier architectural snapshot above used 3,810).
This is not physical FPGA proof. Raw logs/manifests are under
tools/sia/results/2026-10-05-wide-*; check-abi-cross-tu.py --board reproduces it.

All translation units and any hand-written runtime consumers must use this convention consistently. Existing externally authored variadic routines cannot be assumed compatible merely because their fixed scalar calls are compatible. No mixed compiler-version object compatibility is established.

The shared-cursor fixture passes a pointer to va_list before further caller use,
following C variable-argument access semantics in [WG14 draft §7.16](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3220.pdf).
