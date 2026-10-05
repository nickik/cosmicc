# SIA decimal text runtime

Integer-only C-locale `strtof`, `strtod`, `strtold`, `atof`, `snprintf`,
`vsnprintf`, `sprintf` and `vsprintf` implementations. SIA long double uses
binary64 precision. No host floating-point or text callbacks are used by guest code.

Parsing supports decimal/hexadecimal, infinity, NaN, end pointers and ERANGE.
A bounded significant prefix plus sticky information handles long inputs;
exact integer rational rounding supports all four C rounding modes. Formatting
supports narrow strings, integers, pointers, `%n`, and `%f/%e/%g/%a` variants,
width, precision, padding, bounded output and required-length reporting.
Wide output is not implemented; this is memory text I/O, not FILE/console I/O.

Run `python3 runtime/decimal/check.py --output /tmp/decimal-check --random-cases 1000`
from the repository root. The retained [summary](results/2026-10-05/summary.json)
records 16,200 parse bit/end checks, 93,196 format byte/length checks and 28
buffer-boundary checks against glibc in four rounding modes. NaN payloads are
implementation-defined and compared by classification. This is a bounded
reference corpus, not exhaustive certification.

The unchanged C [language fixture](../../tools/sia/decimal-language.c) passed
Lighting execution in 1,440,640 instructions. See retained [guest log](results/2026-10-05/lighting.log).
The runner links explicit SIA runtime objects using reachable-section collection.

The same language fixture passes production CPU/mainboard HDL in Bluesim:
1,440,640 guest instructions, 11,713,002 cycles, 1,655,643 memory backend
transactions, restored stack and no trap. [Production log](results/2026-10-05/production.log).
Rust provides pin transport/scheduling and external RAM backing; this does not
claim a programmed physical FPGA or complete Cosmic boot.
