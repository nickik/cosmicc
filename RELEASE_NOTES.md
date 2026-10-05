# Cosmic C 0.13.0

This version integrates the tested compiler/runtime continuation into master.
It emits freestanding SIA32 COSMIC-SIA objects/images and AMD64 ELF64 objects
and native executables through the existing system linker.

Implemented: SIA I64 arithmetic and ABI, binary32/binary64 software arithmetic
and conversions, binary64-precision long double, task-bound fenv and four
rounding modes, C99 real math families and decimal memory text I/O. Static
multi-object linking and relocation-reachable section collection are included.
AMD64 incoming stdarg and INTEGER/SSE/MEMORY aggregate interoperability pass
GCC/Clang comparisons. C11 generic selection and the documented frontend fixes
are included.

The integrated baseline passes 211/220 native cases at O0/O2 and 146/147
reviewed SIA cases. Numerical fixtures pass production CPU/mainboard RTL;
physical FPGA and full Cosmic boot qualification remain separate.

This release updates exact Git pins between nickik projects only. Rust and
third-party package versions are not intentionally upgraded. Dependency
candidates must preserve System V variadic ABI support and hardware CPU
execution before replacing the previous tested pins.

Open: AMD64 x87 long double, the recorded GNU-extension cases, SIA protected
loader/startup/archive integration and broader hosted library/real-software
qualification. See PORTABILITY_TODO.md and C_COMPATIBILITY_STATUS.md.
