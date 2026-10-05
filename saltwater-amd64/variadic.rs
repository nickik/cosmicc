//! System V variadic calls, with shared aggregate classification and RAX count.
use super::*;

pub(super) struct CallSignature {
    pub signature: Signature,
    pub vector_count: u8,
    pub arguments: Vec<abi::Passing>,
    pub result: abi::ResultKind,
}

pub(super) fn call_signature(ft: &FunctionType, args: &[Expr]) -> Result<CallSignature, Error> {
    let fixed = params(ft);
    if !ft.varargs || args.len() < fixed.len() {
        return Err(unsupported("variadic call argument count"));
    }
    for arg in &args[fixed.len()..] {
        if matches!(
            arg.ctype,
            Type::Float | Type::Bool | Type::Char(_) | Type::SignedChar | Type::Short(_)
        ) {
            return Err(unsupported("unpromoted variadic scalar argument"));
        }
        if matches!(arg.ctype, Type::Array(..) | Type::Function(_)) {
            return Err(unsupported("variadic argument requires pointer decay"));
        }
    }
    let plan = abi::plan(ft, &args[fixed.len()..], true)?;
    let vector_count = plan.fp as u8;
    let mut signature = plan.signature;
    signature.params.push(AbiParam::special(
        types::I32,
        ir::ArgumentPurpose::SystemVVariadicCount,
    ));
    Ok(CallSignature {
        signature,
        vector_count,
        arguments: plan.arguments,
        result: plan.result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outgoing_scalar_variadic_calls_emit_elf() {
        let source = "extern int host(int n,...); int main(void){float f=1.5f; unsigned char c=255; return host(2,c,f);}";
        let bytes = compile(source, Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn incoming_named_only_variadic_definition_emits_elf() {
        let bytes = compile(
            "int incoming(int n,...){return n;} int main(void){return incoming(1,2);}",
            Opt::default(),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
    #[test]
    fn aggregate_variadic_arguments_are_not_misclassified_as_pointers() {
        let bytes=compile("struct S{int x;}; extern int host(int,...); int main(void){struct S s={1};return host(1,s);}",Opt::default()).unwrap();
        assert_eq!(&bytes[..4], b"\x7fELF");
    }
}
