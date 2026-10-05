use saltwater_parser::data::Type;
use saltwater_parser::{check_semantics, Opt, TargetDataModel};

fn check(target: TargetDataModel, source: &str) {
    let result = check_semantics(
        source,
        Opt {
            target,
            ..Opt::default()
        },
    );
    assert!(result.result.is_ok(), "{:?}", result.result.err());
}

#[test]
fn independent_ilp32_and_lp64_sysroots_and_layouts() {
    let body = r#"
#include <stddef.h>
#include <stdint.h>
#include <limits.h>
#include <sys/types.h>
#define REQUIRE(name, value) typedef char name[(value) ? 1 : -1]
REQUIRE(int_bytes, sizeof(int) == 4);
REQUIRE(long_bytes, sizeof(long) == WORD);
REQUIRE(pointer_bytes, sizeof(void *) == WORD);
REQUIRE(size_t_bytes, sizeof(size_t) == WORD);
REQUIRE(ptrdiff_bytes, sizeof(ptrdiff_t) == WORD);
REQUIRE(intptr_bytes, sizeof(intptr_t) == WORD);
REQUIRE(uintptr_bytes, sizeof(uintptr_t) == WORD);
REQUIRE(long_literal_bytes, sizeof(1L) == WORD);
REQUIRE(unsigned_long_literal_bytes, sizeof(1UL) == WORD);
REQUIRE(longlong_bytes, sizeof(long long) == 8);
REQUIRE(long_alignment, _Alignof(long) == WORD);
struct S { char tag; long number; void *ptr; };
REQUIRE(struct_bytes, sizeof(struct S) == WORD * 3);
REQUIRE(struct_alignment, _Alignof(struct S) == WORD);
#if __SIZEOF_LONG__ != WORD || __SIZEOF_POINTER__ != WORD
#error target predefined size mismatch
#endif
#if WORD == 8
#ifndef __x86_64__
#error wrong architecture
#endif
#ifdef __sia32__
#error leaked SIA architecture
#endif
REQUIRE(int_literal, sizeof(1) == 4);
REQUIRE(character_literal, sizeof('a') == 4);
REQUIRE(signed_long_range, LONG_MAX == 9223372036854775807L);
REQUIRE(unsigned_long_range, ULONG_MAX == 18446744073709551615UL);
REQUIRE(promoted_long, sizeof((long)1 + (unsigned int)1) == 8);
REQUIRE(long_shift, (1L << 40) == 1099511627776L);
union U { char bytes[9]; long value; };
REQUIRE(union_padding, sizeof(union U) == 16);
#else
#ifndef __sia32__
#error wrong architecture
#endif
#ifdef __LP64__
#error leaked LP64 data model
#endif
REQUIRE(signed_long_range, LONG_MAX == 2147483647L);
REQUIRE(unsigned_long_range, ULONG_MAX == 4294967295UL);
#endif
"#;
    for target in [
        TargetDataModel::Amd64,
        TargetDataModel::Sia32,
        TargetDataModel::Amd64,
    ] {
        let word = target.word_bytes();
        check(target, &format!("#define WORD {}\n{}", word, body));
    }
}

#[test]
fn explicit_layout_api_preserves_default_sia_sizes() {
    let pointer = Type::Pointer(Box::new(Type::Int(true)), Default::default());
    assert_eq!(pointer.sizeof(), Ok(4));
    assert_eq!(pointer.sizeof_for(TargetDataModel::Amd64), Ok(8));
    assert_eq!(Type::Long(true).alignof_for(TargetDataModel::Amd64), Ok(8));
}

#[test]
fn padded_member_offsets_follow_each_target() {
    let source = "struct S { char tag; long value; void *ptr; };";
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        // Parsing the type through a declared object gives the canonical HIR
        // member identifiers used by both backends.
        let declarations = check_semantics(
            &format!("{} struct S object;", source),
            Opt {
                target,
                ..Opt::default()
            },
        )
        .result
        .unwrap();
        let st = match &declarations.last().unwrap().data.symbol.get().ctype {
            Type::Struct(st) => st.clone(),
            _ => panic!("struct declaration"),
        };
        let members = st.members();
        assert_eq!(st.offset_for(members[0].id, target), 0);
        assert_eq!(st.offset_for(members[1].id, target), target.word_bytes());
        assert_eq!(
            st.offset_for(members[2].id, target),
            target.word_bytes() * 2
        );
    }
}

#[test]
fn amd64_rejects_unrepresented_long_double_layout() {
    for source in [
        "long double object;",
        "typedef long double extended;",
        "long double function(long double argument);",
        "int main(void) { return sizeof(long double); }",
    ] {
        let result = check_semantics(
            source,
            Opt {
                target: TargetDataModel::Amd64,
                ..Opt::default()
            },
        );
        assert!(
            result.result.is_err(),
            "accepted unsupported layout: {}",
            source
        );
    }
    check(TargetDataModel::Sia32, "long double object;");
}

#[test]
fn preprocessing_constant_expressions_short_circuit() {
    let source = "#if 0 != (0 && (0/0))\n#error and\n#endif\n#if 1 != (-1 || (0/0))\n#error or\n#endif\n#if 3 != (-1 ? 3 : (0/0))\n#error conditional\n#endif\n#if 4 != (0 ? (0/0) : 4)\n#error false conditional\n#endif\nint main(void) {return 0;}";
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, source);
    }
}

#[test]
fn runtime_vla_sizeof_survives_constant_folding() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(
            target,
            "int main(void) {int n=3; int values[n]; return sizeof(values) != 3*sizeof(int);}",
        );
    }
}

#[test]
fn argument_prescan_and_macro_line_directives() {
    let source = r#"
#define CAT(x,y) x ## y
#define XCAT(x,y) CAT(x,y)
#define FOO foo
#define BAR bar
#define line 1000
#line line
#if __LINE__ != 1000
#error incorrect remapped line
#endif
#line 1 "virtual.c"
#if __LINE__ != 1
#error incorrect minimum remapped line
#endif
typedef char filename_size[sizeof(__FILE__) == 10 ? 1 : -1];
int main(void) {int foobar=7; return XCAT(FOO,BAR)-7;}
"#;
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, source);
    }
    for bad in ["#line 0\n", "#line 2147483648\n", "#line 2 3\n"] {
        assert!(check_semantics(bad, Opt::default()).result.is_err());
    }
}

#[test]
fn anonymous_members_preserve_nested_storage_and_initializer_boundaries() {
    let source = r#"
struct Inner { int a; int b; };
struct Outer { char tag; union { long value; int alternate; }; struct { struct { long final; }; }; struct Inner tail; };
struct Outer object = {1, 2L, 3L, {4,5}};
int main(void) { object.value++; object.final+=2; return object.tag!=1 || object.value!=3 || object.final!=5 || object.tail.b!=5; }
"#;
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, source);
    }
    assert!(check_semantics("struct S {struct {int duplicate;}; union {long duplicate;};}; int main(void) {struct S s; return s.duplicate;}", Opt::default()).result.is_err());
}

#[test]
fn compound_literals_have_storage_and_inferred_array_bounds() {
    let source = r#"
struct S { int a; long b; };
struct S *global = &(struct S){.a=1, .b=2L};
int main(void) {
  int counter=0;
  struct S *p=&(struct S){counter++, 7L};
  int *a=(int[]){3,4,5};
  return p->a || p->b!=7L || counter!=1 || a[2]!=5 || ((struct S){9,10L}).a!=9 || (int){6}!=6;
}

"#;
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, source);
        for invalid in [
            "int main(void) {(void){0}; return 0;}",
            "int main(void) {int n=2; (int[n]){1}; return 0;}",
            "int main(void) {(const int){0}=1; return 0;}",
        ] {
            assert!(check_semantics(
                invalid,
                Opt {
                    target,
                    ..Opt::default()
                }
            )
            .result
            .is_err());
        }
    }
}

#[test]
fn wide_characters_typedef_shadowing_and_array_parameters() {
    let source = r#"
typedef struct T T;
struct T {int T;};
void bounded(int values[static const 5]);
void bounded(int *values);
void unspecified(int values[volatile *]);
void unspecified(int *values);
void nested(int values[3][*]);
int read(int values[static 4]) {return values[3];}
typedef char wide_width[sizeof(L'\0')==4 ? 1:-1];
typedef char unicode_escape[L'\u03a9'==937 ? 1:-1];
typedef char unicode_source[L'Ω'==937 ? 1:-1];
typedef char wide_octal[L'\777'==511 ? 1:-1];
typedef char wide_hex[L'\x1234'==4660 ? 1:-1];
int main(void) {struct T T; T.T=7; {int T=9; if(T!=9)return 1;} return T.T!=7;}
"#;
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, source);
        for bad in [
            "int invalid[const 3];",
            "void invalid(int p[static *]);",
            "void invalid(int p[2][const 3]);",
            "void invalid(int p[*]) {}",
            "int main(void) {return L'\\uD800';}",
            "int main(void) {return L'\\x100000000';}",
        ] {
            assert!(
                check_semantics(
                    bad,
                    Opt {
                        target,
                        ..Opt::default()
                    }
                )
                .result
                .is_err(),
                "accepted {}",
                bad
            );
        }
    }
}

#[test]
fn c_boolean_operators_have_int_result_type() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        let source = "char c; typedef char A[sizeof(!c)==sizeof(int)?1:-1]; typedef char B[sizeof(c&&c)==sizeof(int)?1:-1]; typedef char C[sizeof(c||c)==sizeof(int)?1:-1]; typedef char D[sizeof(c<c)==sizeof(int)?1:-1]; typedef char E[sizeof(c==c)==sizeof(int)?1:-1]; int main(void){return 0;}";
        let mut opt = Opt::default();
        opt.target = target;
        assert!(check_semantics(source, opt).result.is_ok());
    }
}

#[test]
fn shifts_promote_operands_independently() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, "unsigned int x; long long n; unsigned char c; typedef char A[sizeof(x<<n)==sizeof(x)?1:-1]; typedef char B[sizeof(c<<n)==sizeof(int)?1:-1]; int main(void){return (0x80000000U >> 1LL)!=0x40000000U;}");
    }
}

#[test]
fn floating_suffixes_preserve_c_literal_types() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target,"typedef char A[sizeof(1.0f)==4?1:-1];typedef char B[sizeof(1.0F)==4?1:-1];typedef char C[sizeof(1.0)==8?1:-1];typedef char D[1.000000059604644775390625000001f>1.0f?1:-1];typedef char E[((16777216.0f+1.0f)-16777216.0f)==0.0f?1:-1];int main(void){return 0;}");
        let extended = check_semantics(
            "int main(void){return sizeof(1.0L);}",
            Opt {
                target,
                ..Opt::default()
            },
        );
        assert_eq!(extended.result.is_ok(), target == TargetDataModel::Sia32);
    }
}

#[test]
fn sia_long_double_policy_and_fenv_headers() {
    check(TargetDataModel::Sia32,"#include <float.h>\n#include <fenv.h>\n#pragma STDC FENV_ACCESS ON\ntypedef char A[sizeof(long double)==8?1:-1];typedef char B[sizeof(1.0L)==8?1:-1];typedef char C[LDBL_MANT_DIG==53?1:-1];struct S{char a;long double b;};typedef char D[sizeof(struct S)==16?1:-1];int main(void){long double x=1.0L;return (int)(x+2.0);}");
    let declarations = check_semantics("long double object;", Opt::default())
        .result
        .unwrap();
    assert_eq!(declarations[0].data.symbol.get().ctype, Type::LongDouble);
    assert_ne!(Type::LongDouble, Type::Double);
}

#[test]
fn hexadecimal_integer_candidates_follow_target_width() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, "typedef char A[sizeof(0xffffffff)==4?1:-1];typedef char B[0xffffffff>0?1:-1];typedef char C[sizeof(0xffffffffffffffff)==8?1:-1];typedef char D[0xffffffffffffffff>0?1:-1];typedef char E[0XFFFFFFFF==0xffffffff?1:-1];int main(void){return 0;}");
        check(
            target,
            "struct S{int n;};int main(void){struct S a[1];return a[0].n;}",
        );
    }
}

#[test]
fn character_array_string_initializers_keep_whole_subobjects() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, "struct S{char a[3];int x;};struct S s={\"abc\",4};char a[3]={\"ab\"};char b[][4]={\"abc\",\"de\"};int main(void){struct S local={\"xy\",5};return local.a[0]+s.x+b[1][0];}");
        for source in ["char a[3]=\"abcd\";", "char a[3]=1;", "int a[3]=\"ab\";"] {
            assert!(
                check_semantics(
                    source,
                    Opt {
                        target,
                        ..Opt::default()
                    }
                )
                .result
                .is_err(),
                "accepted {}",
                source
            );
        }
    }
}

#[test]
fn array_lvalues_decay_before_arrow() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(
            target,
            "struct S{int x;};struct S objects[2]={{1},{2}};int main(void){return objects->x;}",
        );
    }
}

#[test]
fn pasted_function_names_rescan_with_invocation_hide_set() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target,"#define CAT2(a,b) a##b\n#define CAT(a,b) CAT2(a,b)\n#define AB(x) CAT(x,y)\nint xy=42;int main(void){return CAT(A,B)(x)!=42;}\n");
    }
}

#[test]
fn wide_strings_preserve_unicode_width_and_concatenation() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, "#include <wchar.h>\nwchar_t s[]=L\"你好¢€\";wchar_t t[3]={L\"ab\"};struct S{wchar_t x[3];int y;};struct S x={L\"ab\",1};typedef char A[sizeof(s)==20?1:-1];typedef char B[sizeof(L\"a\" \"b\" L\"€\")==16?1:-1];int main(void){wchar_t local[]=L\"a'b\\u20ac\\U0001F600\";return sizeof(local)!=24;}");
        assert!(check_semantics(
            "char a[3]=L\"ab\";",
            Opt {
                target,
                ..Opt::default()
            }
        )
        .result
        .is_err());
        assert!(check_semantics(
            "int a[2]=L\"abc\";",
            Opt {
                target,
                ..Opt::default()
            }
        )
        .result
        .is_err());
    }
}

#[test]
fn floating_constant_builtins_are_typed_constant_expressions() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target, "float f=__builtin_inff();double n=__builtin_nan(\"\");typedef char A[sizeof(__builtin_inff())==4?1:-1];typedef char B[sizeof(__builtin_inf())==8?1:-1];int main(void){return 0;}");
    }
    check(
        TargetDataModel::Sia32,
        "long double x=__builtin_infl();long double n=__builtin_nanl(\"\");",
    );
}

#[test]
fn exact_hexadecimal_subnormal_literals_are_accepted() {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        check(target,"double a=0x1p-1074;float b=0x1p-149f;double z=0x0p100;float normal=0x1p-126f;int main(void){return 0;}");
        assert!(check_semantics(
            "double a=0x1p-1075;",
            Opt {
                target,
                ..Opt::default()
            }
        )
        .result
        .is_err());
    }
}
