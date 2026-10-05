use saltwater_parser::{check_semantics, Opt, TargetDataModel};

fn accepts(source: &str) {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
        let result = check_semantics(
            source,
            Opt {
                target,
                ..Opt::default()
            },
        );
        assert!(
            result.result.is_ok(),
            "{target:?}: {:?}",
            result.result.err()
        );
    }
}
fn rejects(source: &str) {
    for target in [TargetDataModel::Sia32, TargetDataModel::Amd64] {
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
            "{}",
            source
        );
    }
}
#[test]
fn conversions_and_distinct_character_types() {
    accepts(
        r#"
#define R(n,v) typedef char n[(v)?1:-1]
R(character_int, sizeof('a')==4 && _Generic('a', int:1, default:0));
R(plain, _Generic((char)0, char:1, signed char:2, unsigned char:3)==1);
R(signed_character, _Generic((signed char)0, char:1, signed char:2, unsigned char:3)==2);
R(unsigned_character, _Generic((unsigned char)0, char:1, signed char:2, unsigned char:3)==3);
R(narrow, _Generic((unsigned short)0, unsigned short:1, int:0)==1);
R(string_decay, _Generic("abc", char*:1, const char*:0)==1);
int fn(int);
R(function_decay, _Generic(fn, int(*)(int):1, default:0)==1);
int test(void) {
 const int i=0; int * const p=0; const int *q=0;
 R(top_const, _Generic(i,int:1,const int:0)==1);
 R(pointer_top_const, _Generic(p,int*:1,int*const:0)==1);
 R(pointer_pointee_const, _Generic(q,int*:0,const int*:1)==1);
 int array[4]; R(array_decay,_Generic(array,int*:1,int[4]:0)==1);
 int x=0; _Generic(i,int:x,default:i)=2;
 return x;
}
"#,
    );
}
#[test]
fn nominal_struct_and_function_compatibility() {
    accepts(
        r#"
#define R(n,v) typedef char n[(v)?1:-1]
struct A {int x;}; struct B {int x;}; typedef struct A Alias;
R(nominal,_Generic((struct A){0},struct A:1,struct B:2)==1);
R(alias,_Generic((Alias){0},struct A:1,default:0)==1);
int f(void); R(return_type,_Generic(f,double(*)():0,int(*)(void):1)==1);
"#,
    );
}
#[test]
fn association_constraints() {
    for source in [
        "int x=_Generic(0,int:1,signed int:2);",
        "int x=_Generic(0,default:1,default:2);",
        "int x=_Generic(0,float:1);",
        "int x=_Generic(0,void:1,default:2);",
        "struct A; int x=_Generic(0,struct A:1,default:2);",
        "int x=_Generic(0,int[]:1,default:2);",
        "int x=_Generic(0,int:1,default:missing);",
        "int x=_Generic(0,int(*)[]:1,int(*)[4]:2,default:3);",
        "int f(int n){return _Generic(0,int(*)[n]:1,default:2);}",
    ] {
        rejects(source);
    }
}
