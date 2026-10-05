use super::PureAnalyzer;
use crate::arch;
use crate::data::types::{ArrayType, StructType};
use crate::data::{hir::*, lex::ComparisonToken, *};
use crate::intern::InternedStr;
use std::convert::TryInto;

impl PureAnalyzer {
    fn compound_literal_writes(
        &mut self,
        storage: Expr,
        init: &Initializer,
        writes: &mut Vec<Expr>,
    ) {
        if storage.ctype.is_scalar() {
            if let Initializer::InitializerList(items) = init {
                if let Some(first) = items.first() {
                    self.compound_literal_writes(storage, first, writes);
                    return;
                }
            }
        }
        let location = storage.location;
        let items = match init {
            Initializer::InitializerList(items) => Some(items),
            _ => None,
        };
        match &storage.ctype {
            Type::Array(element, crate::data::types::ArrayType::Fixed(count)) => {
                let element = (**element).clone();
                for index in 0..*count {
                    let address = Expr {
                        ctype: Type::Pointer(Box::new(element.clone()), Qualifiers::default()),
                        lval: false,
                        location,
                        expr: ExprType::StaticRef(Box::new(storage.clone())),
                    };
                    let offset =
                        literal_for(LiteralValue::UnsignedInt(index), location, self.target);
                    let address = self.pointer_arithmetic(address, offset, &element, location);
                    let member = Expr {
                        ctype: element.clone(),
                        lval: true,
                        location,
                        expr: ExprType::Deref(Box::new(address)),
                    };
                    let string_byte = match init {
                        Initializer::Scalar(e) => match &e.expr {
                            ExprType::Literal(LiteralValue::Str(bytes)) => {
                                let literal = if element.is_char() {
                                    bytes
                                        .get(index as usize)
                                        .map(|byte| LiteralValue::Char(*byte))
                                } else {
                                    bytes.get(index as usize * 4..index as usize * 4 + 4).map(
                                        |word| {
                                            LiteralValue::Int(u32::from_le_bytes(
                                                word.try_into().unwrap(),
                                            )
                                                as i32
                                                as i64)
                                        },
                                    )
                                };
                                literal.map(|literal| {
                                    Initializer::Scalar(Box::new(literal_for(
                                        literal,
                                        location,
                                        self.target,
                                    )))
                                })
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    self.compound_literal_writes(
                        member,
                        string_byte
                            .as_ref()
                            .or_else(|| items.and_then(|items| items.get(index as usize)))
                            .unwrap_or(&Initializer::Zero),
                        writes,
                    );
                }
            }
            Type::Struct(st) | Type::Union(st) => {
                let count = if matches!(storage.ctype, Type::Union(_)) {
                    1
                } else {
                    st.members().len()
                };
                for (index, member) in st.members().iter().take(count).enumerate() {
                    let member = Expr {
                        ctype: member.ctype.clone(),
                        lval: true,
                        location,
                        expr: ExprType::Member(Box::new(storage.clone()), member.id),
                    };
                    self.compound_literal_writes(
                        member,
                        items
                            .and_then(|items| items.get(index))
                            .unwrap_or(&Initializer::Zero),
                        writes,
                    );
                }
            }
            ty if ty.is_scalar() => {
                let value = match init {
                    Initializer::Scalar(value) => (**value).clone(),
                    _ => literal_for(LiteralValue::Int(0), location, self.target)
                        .implicit_cast(ty, &mut self.error_handler),
                };
                writes.push(Expr {
                    ctype: ty.clone(),
                    lval: false,
                    location,
                    expr: ExprType::Binary(BinaryOp::Assign, Box::new(storage), Box::new(value)),
                });
            }
            _ => self.err(
                SemanticError::from("compound literal requires a complete object type"),
                location,
            ),
        }
    }
    /// C11 6.5.1.1. All arms are checked, but only the result expression is
    /// retained, preserving its lvalue/function/void category. Conversion of
    /// the unevaluated control strips only its top-level qualifiers and decays
    /// arrays/functions; integer promotions are deliberately not applied.
    fn generic_selection(
        &mut self,
        control: ast::Expr,
        associations: Vec<(Option<Locatable<ast::TypeName>>, ast::Expr)>,
        location: Location,
    ) -> Expr {
        let control_type = self.expr(control).rval().ctype;
        let mut types = Vec::new();
        let mut selected = None;
        let mut default = None;
        for (typename, expression) in associations {
            let expression_location = expression.location;
            if let Some(typename) = typename {
                let loc = typename.location;
                let typename = typename.data;
                if let Some(id) = typename.declarator.id {
                    self.err(SemanticError::IdInTypeName(id), loc);
                }
                let parsed = self.parse_type(typename.specifiers, typename.declarator.decl, loc);
                if let Some(sc) = parsed.storage_class {
                    self.err(SemanticError::IllegalStorageClass(sc), loc);
                }
                let ty = parsed.ctype;
                let qualifiers = parsed.qualifiers;
                let expression = self.expr(expression);
                if !generic_complete_object(&ty) || generic_variably_modified(&ty) {
                    self.err(SemanticError::from("_Generic association requires a complete object type that is not variably modified"),loc);
                }
                if types
                    .iter()
                    .any(|(other, q)| *q == qualifiers && generic_compatible(other, &ty))
                {
                    self.err(
                        SemanticError::from("_Generic has duplicate compatible association types"),
                        loc,
                    );
                }
                if qualifiers == Qualifiers::NONE && generic_compatible(&control_type, &ty) {
                    if selected.is_some() {
                        self.err(
                            SemanticError::from(
                                "_Generic control matches multiple association types",
                            ),
                            loc,
                        );
                    } else {
                        selected = Some(expression);
                    }
                }
                types.push((ty, qualifiers));
            } else {
                let expression = self.expr(expression);
                if default.is_some() {
                    self.err(
                        SemanticError::from("_Generic has more than one default association"),
                        expression_location,
                    );
                } else {
                    default = Some(expression);
                }
            }
        }
        selected.or(default).unwrap_or_else(|| {
            self.err(
                SemanticError::from(format!(
                    "_Generic control type '{}' has no matching association and no default",
                    control_type
                )),
                location,
            );
            let mut recovery = Expr::zero(location);
            recovery.ctype = Type::Error;
            recovery
        })
    }
    pub fn expr(&mut self, expr: ast::Expr) -> Expr {
        use ast::ExprType::*;

        let _guard = self.recursion_check();
        let _guard2 = self.recursion_check();
        match expr.data {
            // 1 | "str" | 'a'
            Literal(LiteralValue::Char(value)) => {
                let mut literal = literal_for(
                    LiteralValue::Int(i64::from(value)),
                    expr.location,
                    self.target,
                );
                literal.ctype = Type::Int(true);
                literal
            }
            Literal(lit) => literal_for(lit, expr.location, self.target),
            LongLiteral(lit) => {
                let signed = matches!(lit, LiteralValue::Int(_));
                let mut value = literal_for(lit, expr.location, self.target);
                value.ctype = Type::Long(signed);
                value
            }
            NonDecimalLiteral(lit, longs, unsigned) => {
                let raw = match lit {
                    LiteralValue::Int(v) => v as u64,
                    LiteralValue::UnsignedInt(v) => v,
                    _ => unreachable!(),
                };
                let mut value = literal_for(lit, expr.location, self.target);
                let candidates = match (longs, unsigned) {
                    (0, false) => vec![
                        Type::Int(true),
                        Type::Int(false),
                        Type::Long(true),
                        Type::Long(false),
                        Type::LongLong(true),
                        Type::LongLong(false),
                    ],
                    (0, true) => vec![Type::Int(false), Type::Long(false), Type::LongLong(false)],
                    (1, false) => vec![
                        Type::Long(true),
                        Type::Long(false),
                        Type::LongLong(true),
                        Type::LongLong(false),
                    ],
                    (1, true) => vec![Type::Long(false), Type::LongLong(false)],
                    (_, false) => vec![Type::LongLong(true), Type::LongLong(false)],
                    (_, true) => vec![Type::LongLong(false)],
                };
                value.ctype = candidates
                    .into_iter()
                    .find(|ty| {
                        let bits = ty.sizeof_for(self.target).unwrap() * 8;
                        raw <= if ty.is_signed() {
                            if bits == 64 {
                                i64::MAX as u64
                            } else {
                                (1u64 << (bits - 1)) - 1
                            }
                        } else {
                            if bits == 64 {
                                u64::MAX
                            } else {
                                (1u64 << bits) - 1
                            }
                        }
                    })
                    .unwrap();
                value
            }
            FloatLiteral(lit) => {
                let mut value = literal_for(lit, expr.location, self.target);
                value.ctype = Type::Float;
                value
            }
            LongDoubleLiteral(lit) => {
                if self.target == crate::TargetDataModel::Amd64 {
                    self.err(
                        SemanticError::Generic(
                            "AMD64 long double literal ABI is unsupported".into(),
                        ),
                        expr.location,
                    );
                }
                let mut value = literal_for(lit, expr.location, self.target);
                value.ctype = Type::LongDouble;
                value
            }
            WideStringLiteral(lit) => {
                let mut value = literal_for(lit, expr.location, self.target);
                if let ExprType::Literal(LiteralValue::Str(bytes)) = &value.expr {
                    value.ctype = Type::Array(
                        Box::new(Type::Int(true)),
                        crate::data::types::ArrayType::Fixed((bytes.len() / 4) as u64),
                    );
                }
                value
            }
            WideCharLiteral(lit) => {
                let mut value = literal_for(lit, expr.location, self.target);
                value.ctype = Type::Int(true);
                value
            }
            LongLongLiteral(lit) => {
                let signed = matches!(lit, LiteralValue::Int(_));
                let mut value = literal_for(lit, expr.location, self.target);
                value.ctype = Type::LongLong(signed);
                value
            }
            GenericSelection(control, associations) => {
                self.generic_selection(*control, associations, expr.location)
            }
            // x
            Id(id) => self.parse_id(id, expr.location),
            // (int)x
            Cast(ctype, inner) => {
                let ctype = self.parse_typename(ctype, expr.location);
                self.explicit_cast(*inner, ctype)
            }
            CompoundLiteral(ctype, initializer) => {
                if let Some(id) = ctype.declarator.id {
                    self.err(SemanticError::IdInTypeName(id), expr.location);
                }
                let parsed =
                    self.parse_type(ctype.specifiers, ctype.declarator.decl, expr.location);
                if let Some(class) = parsed.storage_class {
                    self.err(SemanticError::IllegalStorageClass(class), expr.location);
                }
                let qualifiers = parsed.qualifiers;
                let mut ctype = parsed.ctype;
                fn valid_object(ty: &Type, target: crate::TargetDataModel) -> bool {
                    match ty {
                        Type::Void | Type::Function(_) | Type::Error => false,
                        Type::Array(_, crate::data::types::ArrayType::Variable(_)) => false,
                        Type::Array(element, _) => valid_object(element, target),
                        _ => ty.sizeof_for(target).is_ok(),
                    }
                }
                if !valid_object(&ctype, self.target) {
                    self.err(SemanticError::from("compound literal requires a complete object type or an unknown-bound array, and cannot have variable length"), expr.location);
                    let mut recovery = Expr::zero(expr.location);
                    recovery.ctype = Type::Error;
                    return recovery;
                }
                let init = self.parse_initializer(*initializer, &ctype, expr.location);
                if let Type::Array(_, bound @ crate::data::types::ArrayType::Unbounded) = &mut ctype
                {
                    if let Initializer::InitializerList(items) = &init {
                        *bound = crate::data::types::ArrayType::Fixed(items.len() as u64);
                    }
                }
                let global = self.scope.is_global();
                let id = format!("$compound{}", self.compound_literal_count).into();
                self.compound_literal_count += 1;
                let symbol = self.declare(
                    Variable {
                        id,
                        ctype: ctype.clone(),
                        qualifiers,
                        storage_class: if global {
                            StorageClass::Static
                        } else {
                            StorageClass::Auto
                        },
                    },
                    true,
                    expr.location,
                );
                let storage = Expr {
                    ctype,
                    lval: true,
                    location: expr.location,
                    expr: ExprType::Id(symbol),
                };
                if global {
                    self.pending.push_back(expr.location.with(Declaration {
                        symbol,
                        init: Some(init),
                    }));
                    storage
                } else {
                    self.decl_side_channel
                        .push(expr.location.with(Declaration { symbol, init: None }));
                    let mut writes = Vec::new();
                    self.compound_literal_writes(storage.clone(), &init, &mut writes);
                    let location = expr.location;
                    let address = Expr {
                        ctype: Type::Pointer(Box::new(storage.ctype.clone()), qualifiers),
                        lval: false,
                        location,
                        expr: ExprType::StaticRef(Box::new(storage.clone())),
                    };
                    let initialized_address =
                        writes.into_iter().rev().fold(address, |tail, write| Expr {
                            ctype: tail.ctype.clone(),
                            lval: false,
                            location,
                            expr: ExprType::Comma(Box::new(write), Box::new(tail)),
                        });
                    // Existing HIR's pointer-valued Noop denotes an lvalue's
                    // address without introducing an additional load.
                    Expr {
                        ctype: storage.ctype,
                        lval: true,
                        location,
                        expr: ExprType::Noop(Box::new(initialized_address)),
                    }
                }
            }
            Shift(left, right, direction) => {
                let op = if direction {
                    BinaryOp::Shl
                } else {
                    BinaryOp::Shr
                };
                self.binary_helper(left, right, op, Self::parse_integer_op)
            }
            BitwiseAnd(left, right) => {
                self.binary_helper(left, right, BinaryOp::BitwiseAnd, Self::parse_integer_op)
            }
            BitwiseOr(left, right) => {
                self.binary_helper(left, right, BinaryOp::BitwiseOr, Self::parse_integer_op)
            }
            Xor(left, right) => {
                self.binary_helper(left, right, BinaryOp::Xor, Self::parse_integer_op)
            }
            Compare(left, right, token) => self.relational_expr(*left, *right, token),
            Mul(left, right) => self.binary_helper(left, right, BinaryOp::Mul, Self::mul),
            Div(left, right) => self.binary_helper(left, right, BinaryOp::Div, Self::mul),
            Mod(left, right) => self.binary_helper(left, right, BinaryOp::Mod, Self::mul),
            Assign(lval, rval, token) => {
                let lval = self.expr(*lval);
                let rval = self.expr(*rval);
                self.assignment_expr(lval, rval, token, expr.location)
            }
            Add(left, right) => self.binary_helper(left, right, BinaryOp::Add, Self::add),
            Sub(left, right) => self.binary_helper(left, right, BinaryOp::Sub, Self::add),
            FuncCall(func, args) => {
                let builtin = match &func.data {
                    ast::ExprType::Id(id) => match id.resolve_and_clone().as_str() {
                        "__builtin_inf" | "__builtin_inff" | "__builtin_infl" => {
                            Some((f64::INFINITY, false))
                        }
                        "__builtin_nan" | "__builtin_nanf" | "__builtin_nanl" => {
                            Some((f64::NAN, true))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((number, nan)) = builtin {
                    let name = match &func.data {
                        ast::ExprType::Id(id) => id.resolve_and_clone(),
                        _ => unreachable!(),
                    };
                    let valid = if nan {
                        args.len() == 1
                            && matches!(&args[0].data, ast::ExprType::Literal(LiteralValue::Str(bytes)) if bytes == &[0])
                    } else {
                        args.is_empty()
                    };
                    if !valid {
                        self.err(SemanticError::from("floating constant builtin requires no arguments (infinity) or an empty string (NaN)"),expr.location);
                    }
                    let mut value =
                        literal_for(LiteralValue::Float(number), expr.location, self.target);
                    if name.ends_with('f') {
                        value.ctype = Type::Float;
                    }
                    if name.ends_with('l') {
                        if self.target == crate::TargetDataModel::Amd64 {
                            self.err(
                                SemanticError::from("AMD64 long double literal ABI is unsupported"),
                                expr.location,
                            );
                        }
                        value.ctype = Type::LongDouble;
                    }
                    value
                } else {
                    self.func_call(*func, args)
                }
            }
            Member(struct_, id) => {
                let struct_ = self.expr(*struct_);
                self.struct_member(struct_, id, expr.location)
            }
            // s->p desguars to (*s).p
            DerefMember(inner, id) => {
                let inner = self.expr(*inner).rval();
                let struct_type = match &inner.ctype {
                    Type::Pointer(ctype, _) => match &**ctype {
                        Type::Union(_) | Type::Struct(_) => (**ctype).clone(),
                        Type::Error => return inner,
                        other => {
                            self.err(SemanticError::NotAStruct(other.clone()), inner.location);
                            return inner;
                        }
                    },
                    Type::Error => return inner,
                    other => {
                        self.err(SemanticError::NotAPointer(other.clone()), inner.location);
                        return inner;
                    }
                };
                // NOTE: when we pass `deref` to `struct_member`,
                // it will always mark the resulting expression as an `lval`.
                // To avoid a double dereference, we mark `deref` as an `rval`.
                let deref = inner.indirection(false, struct_type);
                self.struct_member(deref, id, expr.location)
            }
            // `*p` or `a[i]`
            Deref(inner) => {
                let inner = self.expr(*inner);
                match &inner.ctype {
                    Type::Array(t, _) | Type::Pointer(t, _) => {
                        let ctype = (**t).clone();
                        inner.indirection(true, ctype)
                    }
                    Type::Error => inner,
                    _ => {
                        self.err(
                            SemanticError::NotAPointer(inner.ctype.clone()),
                            expr.location,
                        );
                        inner
                    }
                }
            }
            // &x
            // 6.5.3.2 Address and indirection operators
            AddressOf(inner) => {
                let inner = self.expr(*inner);
                match inner.expr {
                    // parse &*x as x
                    // footnote 102: &*E is equivalent to E (even if E is a null pointer)
                    ExprType::Deref(double_inner) => *double_inner,
                    // footnote 121:
                    // > the address of any part of an object declared with storage-class specifier register cannot be computed,
                    // > either explicitly (by use of the unary & operator as discussed in 6.5.3.2)
                    // > or implicitly (by converting an array name to a pointer as discussed in 6.3.2.1).
                    ExprType::Id(ref sym) if sym.get().storage_class == StorageClass::Register => {
                        self.err(
                            SemanticError::InvalidAddressOf("variable declared with `register`"),
                            expr.location,
                        );
                        inner
                    }
                    // > The operand of the unary & operator shall be either a function designator,
                    // > the result of a [] or unary * operator,
                    // > or an lvalue that designates an object that is not a bit-field and is not declared with the register storage-class specifier.
                    _ if inner.lval => Expr {
                        lval: false,
                        location: expr.location,
                        ctype: Type::Pointer(
                            Box::new(inner.ctype.clone()),
                            inner.lvalue_qualifiers(),
                        ),
                        expr: ExprType::StaticRef(Box::new(inner)),
                    },
                    _ => {
                        self.err(SemanticError::InvalidAddressOf("value"), expr.location);
                        inner
                    }
                }
            }
            // ++x
            PreIncrement(inner, increment) => {
                self.increment_op(true, increment, *inner, expr.location)
            }
            // x++
            PostIncrement(inner, increment) => {
                self.increment_op(false, increment, *inner, expr.location)
            }
            // a[i]
            Index(left, right) => self.index(*left, *right, expr.location),
            AlignofType(type_name) => {
                let ctype = self.parse_typename(type_name, expr.location);
                self.align(ctype, expr.location)
            }
            AlignofExpr(inner) => {
                let inner = self.expr(*inner);
                self.align(inner.ctype, expr.location)
            }
            SizeofType(type_name) => {
                let ctype = self.parse_typename(type_name, expr.location);
                self.sizeof(ctype, expr.location)
            }
            SizeofExpr(inner) => {
                let inner = self.expr(*inner);
                self.sizeof(inner.ctype, expr.location)
            }
            BitwiseNot(inner) => self.bitwise_not(*inner),
            UnaryPlus(inner) => self.unary_add(*inner, true, expr.location),
            Negate(inner) => self.unary_add(*inner, false, expr.location),
            // !x
            LogicalNot(inner) => self.logical_not(*inner),
            // x && y
            LogicalAnd(left, right) => {
                self.binary_helper(left, right, BinaryOp::LogicalAnd, Self::logical_bin_op)
            }
            // x || y
            LogicalOr(left, right) => {
                self.binary_helper(left, right, BinaryOp::LogicalOr, Self::logical_bin_op)
            }
            // x, y
            // evaluate x, discarding its value, then yield the value of y
            // mostly used to have multiple side effects in a single statement, such as in a for loop:
            // `for(j = i, k = 0; k < n; j++, k++);`
            // see also https://stackoverflow.com/a/43561555/7669110
            Comma(left, right) => {
                let left = self.expr(*left);
                let right = self.expr(*right).rval();
                Expr {
                    ctype: right.ctype.clone(),
                    lval: false,
                    expr: ExprType::Comma(Box::new(left), Box::new(right)),
                    location: expr.location,
                }
            }
            Ternary(condition, then, otherwise) => {
                self.ternary(*condition, *then, *otherwise, expr.location)
            }
        }
    }
    // only meant for use with `expr`
    // TODO: change ast::Expr to use `ExprType::Binary` as well, which would make this unnecessary
    // TODO: these functions should have the locations of the parent expression, not the children
    #[allow(clippy::boxed_local)]
    fn binary_helper<F>(
        &mut self,
        left: Box<ast::Expr>,
        right: Box<ast::Expr>,
        op: BinaryOp,
        expr_checker: F,
    ) -> Expr
    where
        F: FnOnce(&mut Self, Expr, Expr, BinaryOp) -> Expr,
    {
        let left = self.expr(*left);
        let right = self.expr(*right);
        expr_checker(self, left, right, op)
    }
    // left OP right, where OP is an operation that requires integral types
    fn parse_integer_op(&mut self, left: Expr, right: Expr, op: BinaryOp) -> Expr {
        let non_scalar = if !left.ctype.is_integral() {
            Some(&left.ctype)
        } else if !right.ctype.is_integral() {
            Some(&right.ctype)
        } else {
            None
        };
        let location = left.location.merge(right.location);
        if let Some(ctype) = non_scalar {
            // if not already type error
            if *ctype != Type::Error {
                self.err(SemanticError::NonIntegralExpr(ctype.clone()), location);
            }
        }
        // C shifts promote operands independently; the right operand cannot
        // change the width or signedness of the left operand or result.
        let (promoted_expr, next) = if matches!(op, BinaryOp::Shl | BinaryOp::Shr) {
            (
                left.integer_promote(&mut self.error_handler),
                right.integer_promote(&mut self.error_handler),
            )
        } else {
            Expr::binary_promote_for(left, right, &mut self.error_handler, self.target)
        };
        Expr {
            ctype: promoted_expr.ctype.clone(),
            expr: ExprType::Binary(op, Box::new(promoted_expr), Box::new(next)),
            lval: false,
            location,
        }
    }
    // x
    fn parse_id(&mut self, name: InternedStr, location: Location) -> Expr {
        let mut pretend_zero = Expr::zero(location);
        pretend_zero.ctype = Type::Error;
        pretend_zero.lval = true; // set undeclared identifier as lval
        match self.scope.get(&name) {
            None => {
                self.err(SemanticError::UndeclaredVar(name), location);
                pretend_zero
            }
            Some(&symbol) => {
                let meta = symbol.get();
                // typedef int i; return i + 1;
                if meta.storage_class == StorageClass::Typedef {
                    self.err(SemanticError::TypedefInExpressionContext, location);
                    return pretend_zero;
                }
                if let Type::Enum(ident, members) = &meta.ctype {
                    let mapper = |(member, value): &(InternedStr, i64)| {
                        if name == *member {
                            Some(*value)
                        } else {
                            None
                        }
                    };
                    let enumerator = members.iter().find_map(mapper);
                    // enum e { A }; return A;
                    if let Some(e) = enumerator {
                        return Expr {
                            ctype: Type::Enum(*ident, members.clone()),
                            location,
                            lval: false,
                            expr: ExprType::Literal(LiteralValue::Int(e)),
                        };
                    }
                    // otherwise, `enum e { A } my_e; return my_e;`
                }
                Expr::id(symbol, location)
            }
        }
    }
    // `left == right`, `left < right`, or similar
    // 6.5.9 Equality operators
    fn relational_expr(
        &mut self,
        left: ast::Expr,
        right: ast::Expr,
        token: ComparisonToken,
    ) -> Expr {
        let location = left.location.merge(right.location);
        let mut left = self.expr(left);
        let mut right = self.expr(right);

        // i == i
        if left.ctype.is_arithmetic() && right.ctype.is_arithmetic() {
            let tmp = Expr::binary_promote_for(left, right, &mut self.error_handler, self.target);
            left = tmp.0;
            right = tmp.1;
        } else {
            let (left_expr, right_expr) = (left.rval(), right.rval());
            // p1 == p2
            // first check that not already type error
            if left_expr.ctype != Type::Error
                && right_expr.ctype != Type::Error // Maybe I should pull this out of the if...
                && !(compatible_pointer_types(&left_expr.ctype, &right_expr.ctype)
                // equality operations have different rules :(
                || ((token == ComparisonToken::EqualEqual || token == ComparisonToken::NotEqual)
                    // shoot me now
                    // (int*)p1 == (void*)p2
                    && ((left_expr.ctype.is_pointer() && right_expr.ctype.is_void_pointer())
                        // (void*)p1 == (int*)p2
                        || (left_expr.ctype.is_void_pointer() && right_expr.ctype.is_pointer())
                        // NULL == (int*)p2
                        || (left_expr.is_null() && right_expr.ctype.is_pointer())
                        // (int*)p1 == NULL
                        || (left_expr.ctype.is_pointer() && right_expr.is_null()))))
            {
                self.err(
                    SemanticError::InvalidRelationalType(
                        token,
                        left_expr.ctype.clone(),
                        right_expr.ctype.clone(),
                    ),
                    location,
                );
            }
            left = left_expr;
            right = right_expr;
        }
        assert!(!left.lval && !right.lval);
        Expr {
            lval: false,
            location,
            ctype: Type::Int(true),
            expr: ExprType::Binary(BinaryOp::Compare(token), Box::new(left), Box::new(right)),
        }
    }
    // `left OP right`, where OP is Mul, Div, or Mod
    // 6.5.5 Multiplicative operators
    fn mul(&mut self, left: Expr, right: Expr, op: BinaryOp) -> Expr {
        let location = left.location.merge(right.location);

        if left.ctype == Type::Error {
            return left;
        } else if right.ctype == Type::Error {
            return right;
        }

        if op == BinaryOp::Mod && !(left.ctype.is_integral() && right.ctype.is_integral()) {
            self.err(
                SemanticError::from(format!(
                    "expected integers for both operators of %, got '{}' and '{}'",
                    left.ctype, right.ctype
                )),
                location,
            );
        } else if !(left.ctype.is_arithmetic() && right.ctype.is_arithmetic()) {
            self.err(
                SemanticError::from(format!(
                    "expected float or integer types for both operands of {}, got '{}' and '{}'",
                    op, left.ctype, right.ctype
                )),
                location,
            );
        }
        let (left, right) =
            Expr::binary_promote_for(left, right, &mut self.error_handler, self.target);
        Expr {
            ctype: left.ctype.clone(),
            location,
            lval: false,
            expr: ExprType::Binary(op, Box::new(left), Box::new(right)),
        }
    }
    // `a + b` or `a - b`
    // `op` should only be `Add` or `Sub`
    // 6.5.6 Additive operators
    fn add(&mut self, mut left: Expr, mut right: Expr, op: BinaryOp) -> Expr {
        let is_add = op == BinaryOp::Add;
        let location = left.location.merge(right.location);
        // Decay array operands before pointer compatibility/difference checks.
        left = left.rval();
        right = right.rval();
        match (&left.ctype, &right.ctype) {
            // `p + i`
            (Type::Pointer(to, _), i)
            | (Type::Array(to, _), i) if i.is_integral() && to.is_complete() => {
                let to = to.clone();
                let (left, right) = (left.rval(), right.rval());
                let mut result = self.pointer_arithmetic(left, right, &*to, location);
                if !is_add {
                    if let ExprType::Binary(operator, _, _) = &mut result.expr {
                        *operator = BinaryOp::Sub;
                    }
                }
                return result;
            }
            // `i + p`
            (i, Type::Pointer(to, _))
                // `i - p` for pointer p is not valid
            | (i, Type::Array(to, _)) if i.is_integral() && is_add && to.is_complete() => {
                let to = to.clone();
                let (left, right) = (left.rval(), right.rval());
                return self.pointer_arithmetic(right, left, &*to, location);
            }
            _ => {}
        };
        // `i + i`
        let (ctype, lval) = if left.ctype.is_arithmetic() && right.ctype.is_arithmetic() {
            let tmp = Expr::binary_promote_for(left, right, &mut self.error_handler, self.target);
            left = tmp.0;
            right = tmp.1;
            (left.ctype.clone(), false)
        // `p1 - p2`
        // `p1 + p2` for pointers p1 and p2 is not valid
        } else if !is_add
            && left.ctype.is_pointer_to_complete_object()
            && compatible_pointer_types(&left.ctype, &right.ctype)
        {
            // C11 6.5.6: subtracting two compatible object pointers yields
            // ptrdiff_t. SIA32 uses a 32-bit signed long for this frontend
            // representation; importantly, the result is not pointer-typed.
            (Type::Long(true), false)
        } else {
            // check if already type error
            if left.ctype != Type::Error && right.ctype != Type::Error {
                self.err(
                    SemanticError::InvalidAdd(op, left.ctype.clone(), right.ctype.clone()),
                    location,
                );
            }
            (left.ctype.clone(), false)
        };
        Expr {
            ctype,
            lval,
            location,
            expr: ExprType::Binary(op, Box::new(left), Box::new(right)),
        }
    }
    // (int)i
    // 6.5.4 Cast operators
    fn explicit_cast(&mut self, expr: ast::Expr, ctype: Type) -> Expr {
        let location = expr.location;
        let expr = self.expr(expr).rval();
        // (void)0;
        if ctype == Type::Void {
            // casting anything to void is allowed
            return Expr {
                lval: false,
                ctype,
                // this just signals to the backend to ignore this outer expr
                expr: ExprType::Cast(Box::new(expr)),
                location,
            };
        }
        // (struct s)1
        if !ctype.is_scalar() {
            self.err(SemanticError::NonScalarCast(ctype.clone()), location);
        // (int*)1.0
        } else if expr.ctype.is_floating() && ctype.is_pointer()
            // (float)(int*)p
            || expr.ctype.is_pointer() && ctype.is_floating()
        {
            self.err(SemanticError::FloatPointerCast(ctype.clone()), location);
        // struct { int i; } s; (int)s
        } else if expr.ctype.is_struct() {
            // not implemented: galaga (https://github.com/jyn514/rcc/issues/98)
            self.err(SemanticError::StructCast, location);
        // void f(); (int)f();
        } else if expr.ctype == Type::Void {
            self.err(SemanticError::VoidCast, location);
        }
        Expr {
            lval: false,
            expr: ExprType::Cast(Box::new(expr)),
            ctype,
            location,
        }
    }
    // `base + index`, where `pointee` is the type of `*base`
    // 6.5.6 Additive operators
    fn pointer_arithmetic(
        &mut self,
        base: Expr,
        index: Expr,
        pointee: &Type,
        location: Location,
    ) -> Expr {
        // the idea is to desugar to `base + sizeof(base)*index`
        // Keep the index in its integer domain. Older lowering cast the
        // integer index to the pointer type before multiplying by sizeof(T),
        // which later forced an invalid pointer-to-integer implicit cast.
        // Scale in the target's pointer-sized integer domain. In particular,
        // byte/short indices must not truncate the byte offset during Mul.
        let offset = index
            .rval()
            .implicit_cast(&Type::Long(true), &mut self.error_handler);
        let size_expr = match pointee {
            Type::Array(element, crate::data::types::ArrayType::Variable(bound_expression)) => {
                let element_size = match element.sizeof_for(self.target) {
                    Ok(size) => size,
                    Err(_) => {
                        self.err(
                            SemanticError::PointerAddUnknownSize(base.ctype.clone()),
                            location,
                        );
                        1
                    }
                };
                let element_size =
                    literal(LiteralValue::UnsignedInt(element_size), offset.location)
                        .implicit_cast(&offset.ctype, &mut self.error_handler);
                let bound = (**bound_expression)
                    .clone()
                    .implicit_cast(&offset.ctype, &mut self.error_handler);
                Expr {
                    lval: false,
                    location: offset.location,
                    ctype: offset.ctype.clone(),
                    expr: ExprType::Binary(BinaryOp::Mul, Box::new(element_size), Box::new(bound)),
                }
            }
            _ => {
                let size = match pointee.sizeof_for(self.target) {
                    Ok(size) => size,
                    Err(_) => {
                        self.err(
                            SemanticError::PointerAddUnknownSize(base.ctype.clone()),
                            location,
                        );
                        1
                    }
                };
                literal(LiteralValue::UnsignedInt(size), offset.location)
                    .implicit_cast(&offset.ctype, &mut self.error_handler)
            }
        };
        let offset = Expr {
            lval: false,
            location: offset.location,
            ctype: offset.ctype.clone(),
            expr: ExprType::Binary(BinaryOp::Mul, Box::new(size_expr), Box::new(offset)),
        };
        Expr {
            lval: false,
            location,
            ctype: base.ctype.clone(),
            expr: ExprType::Binary(BinaryOp::Add, Box::new(base), Box::new(offset)),
        }
    }
    // `func(args)`
    // 6.5.2.2 Function calls
    fn func_call(&mut self, func: ast::Expr, args: Vec<ast::Expr>) -> Expr {
        let mut func = self.expr(func);
        // if fp is a function pointer, fp() desugars to (*fp)()
        match &func.ctype {
            Type::Pointer(pointee, _) if pointee.is_function() => {
                let ctype = (**pointee).clone();
                func = Expr {
                    lval: false,
                    location: func.location,
                    ctype,
                    expr: ExprType::Deref(Box::new(func.rval())),
                }
            }
            _ => {}
        };
        let functype = match &func.ctype {
            Type::Function(functype) => functype,
            Type::Error => return func, // we've already reported this error
            other => {
                self.err(SemanticError::NotAFunction(other.clone()), func.location);
                return func;
            }
        };
        let mut expected = functype.params.len();
        // f(void)
        if expected == 1 && functype.params[0].get().ctype == Type::Void {
            expected = 0;
        }
        // f() takes _any_ number of arguments
        if !functype.params.is_empty()
            // `int f(int); f()` or `int f(int); f(1, 2)`
            && (args.len() < expected || args.len() > expected && !functype.varargs)
        {
            self.err(
                SemanticError::WrongArgumentNumber(args.len(), expected),
                func.location,
            );
        }
        let mut promoted_args = vec![];
        for (i, arg) in args.into_iter().enumerate() {
            let arg = self.expr(arg);
            let promoted = match functype.params.get(i) {
                // int f(int); f(1)
                Some(expected) => arg
                    .rval()
                    .implicit_cast(&expected.get().ctype, &mut self.error_handler),
                // `int f(); f(1)` or `int f(int, ...); f(1, 2)`
                None => self.default_promote(arg),
            };
            promoted_args.push(promoted);
        }
        Expr {
            location: func.location,
            lval: false, // no move semantics here!
            ctype: *functype.return_type.clone(),
            expr: ExprType::FuncCall(Box::new(func), promoted_args),
        }
    }
    /// 'default promotions' from 6.5.2.2p6
    fn default_promote(&mut self, expr: Expr) -> Expr {
        let expr = expr.rval();
        let ctype = expr.ctype.clone().default_promote();
        expr.implicit_cast(&ctype, &mut self.error_handler)
    }
    // parse a struct member
    // used for both s.a and s->a
    // 6.5.2.3 Structure and union members
    fn struct_member(&mut self, expr: Expr, id: InternedStr, location: Location) -> Expr {
        match &expr.ctype {
            Type::Struct(stype) | Type::Union(stype) => {
                let members = stype.members();
                // struct s; s.a
                if members.is_empty() {
                    self.err(
                        SemanticError::IncompleteDefinitionUsed(expr.ctype.clone()),
                        location,
                    );
                    expr
                // struct s { int i; }; s.i
                } else if let Some(member) = members.iter().find(|member| member.id == id) {
                    Expr {
                        ctype: member.ctype.clone(),
                        lval: true,
                        location,
                        expr: ExprType::Member(Box::new(expr), id),
                    }
                // struct s { int i; }; s.j
                } else {
                    fn promoted_paths(ty: &Type, name: InternedStr) -> Vec<Vec<InternedStr>> {
                        let st = match ty {
                            Type::Struct(st) | Type::Union(st) => st,
                            _ => return Vec::new(),
                        };
                        let mut paths = Vec::new();
                        for member in st.members().iter() {
                            if member.id == name {
                                paths.push(vec![name]);
                            } else if member.id.resolve_and_clone().starts_with("$anonymous") {
                                for mut path in promoted_paths(&member.ctype, name) {
                                    path.insert(0, member.id);
                                    paths.push(path);
                                }
                            }
                        }
                        paths
                    }
                    let paths = promoted_paths(&expr.ctype, id);
                    if paths.len() == 1 {
                        let mut result = expr;
                        for field in &paths[0] {
                            result = self.struct_member(result, *field, location);
                        }
                        result
                    } else {
                        if paths.len() > 1 {
                            self.err(
                                SemanticError::from(
                                    "ambiguous member promoted by anonymous struct or union",
                                ),
                                location,
                            );
                        } else {
                            self.err(SemanticError::NotAMember(id, expr.ctype.clone()), location);
                        }
                        expr
                    }
                }
            }
            // if already type error
            Type::Error => expr,
            // (1).a
            _ => {
                self.err(SemanticError::NotAStruct(expr.ctype.clone()), location);
                expr
            }
        }
    }
    // ++i, i--
    // 6.5.2.4 Postfix increment and decrement operators
    fn increment_op(
        &mut self,
        prefix: bool,
        increment: bool,
        expr: ast::Expr,
        location: Location,
    ) -> Expr {
        use crate::data::lex::AssignmentToken;

        let expr = self.expr(expr);
        if let Err(err) = expr.modifiable_lval() {
            self.err(err, location);
        } else if !(expr.ctype.is_arithmetic() || expr.ctype.is_pointer()) {
            // check if already encountered type error
            if expr.ctype != Type::Error {
                self.err(
                    SemanticError::InvalidIncrement(expr.ctype.clone()),
                    expr.location,
                );
            }
        }
        // ++i is syntactic sugar for i+=1
        if prefix {
            // The increment operand is integer one even for a pointer.
            // Casting it to the lvalue type turns ++p into invalid p + p.
            let rval = literal(LiteralValue::Int(1), location);
            let op = if increment {
                AssignmentToken::AddEqual
            } else {
                AssignmentToken::SubEqual
            };
            self.assignment_expr(expr, rval, op, location)
        // i++ requires support from the backend
        // 6.5.2.4 Postfix increment and decrement operators
        // evaluate the rvalue of `i` and as a side effect, increment the value at the stored address
        // ex: `int i = 0, j; j = i++;` leaves a value of 0 in j and a value of 1 in i
        } else {
            Expr {
                lval: false,
                ctype: expr.ctype.clone(),
                // true, false: increment/decrement
                expr: ExprType::PostIncrement(Box::new(expr), increment),
                location,
            }
        }
    }
    // a[i] desugars to *(a + i)
    // 6.5.2.1 Array subscripting
    fn index(&mut self, left: ast::Expr, right: ast::Expr, location: Location) -> Expr {
        let left = self.expr(left).rval();
        let right = self.expr(right).rval();

        let (target_type, array, index) = match (&left.ctype, &right.ctype) {
            // p[i]
            (Type::Pointer(target, _), _) => ((**target).clone(), left, right),
            // i[p]
            (_, Type::Pointer(target, _)) => ((**target).clone(), right, left),
            // already type error
            (Type::Error, _) => return left,
            (_, Type::Error) => return right,
            (l, _) => {
                self.err(SemanticError::NotAPointer(l.clone()), location);
                return left;
            }
        };
        let mut addr = self.pointer_arithmetic(array, index, &target_type, location);
        addr.ctype = target_type;
        // `p + i` -> `*(p + i)`
        addr.lval = true;
        addr
    }
    // _Alignof(int)
    fn align(&mut self, ctype: Type, location: Location) -> Expr {
        let align = ctype.alignof_for(self.target).unwrap_or_else(|err| {
            self.err(err.into(), location);
            1
        });
        let mut result = literal_for(LiteralValue::UnsignedInt(align), location, self.target);
        result.ctype = Type::Long(false); // target size_t
        result
    }
    // sizeof(int)
    // 6.5.3.4 The sizeof and _Alignof operators
    fn sizeof(&mut self, ctype: Type, location: Location) -> Expr {
        if matches!(
            ctype,
            Type::Array(_, crate::data::types::ArrayType::Variable(_))
        ) {
            return Expr {
                lval: false,
                ctype: Type::Long(false),
                location,
                expr: ExprType::Sizeof(ctype),
            };
        }
        let size = ctype.sizeof_for(self.target).unwrap_or_else(|err| {
            if ctype != Type::Error {
                self.err(err.into(), location);
            }
            1
        });
        let mut result = literal_for(LiteralValue::UnsignedInt(size), location, self.target);
        result.ctype = Type::Long(false); // target size_t
        result
    }
    // ~expr
    // 6.5.3.3 Unary arithmetic operators
    fn bitwise_not(&mut self, expr: ast::Expr) -> Expr {
        let expr = self.expr(expr);
        if !expr.ctype.is_integral() {
            // check if already error
            if expr.ctype != Type::Error {
                self.err(
                    SemanticError::NonIntegralExpr(expr.ctype.clone()),
                    expr.location,
                );
            }
            expr
        } else {
            let expr = expr.integer_promote(&mut self.error_handler);
            Expr {
                lval: false,
                ctype: expr.ctype.clone(),
                location: expr.location,
                expr: ExprType::BitwiseNot(Box::new(expr)),
            }
        }
    }
    // -x and +x
    // 6.5.3.3 Unary arithmetic operators
    fn unary_add(&mut self, expr: ast::Expr, add: bool, location: Location) -> Expr {
        let expr = self.expr(expr);
        if !expr.ctype.is_arithmetic() {
            // check if already error
            if expr.ctype != Type::Error {
                self.err(SemanticError::NotArithmetic(expr.ctype.clone()), location);
            }
            return expr;
        }
        let expr = expr.integer_promote(&mut self.error_handler);
        if add {
            Expr {
                lval: false,
                location,
                ..expr
            }
        } else {
            Expr {
                lval: false,
                ctype: expr.ctype.clone(),
                location,
                expr: ExprType::Negate(Box::new(expr)),
            }
        }
    }
    // !expr
    // 6.5.3.3 Unary arithmetic operators
    // > The expression !E is equivalent to (0==E).
    fn logical_not(&mut self, expr: ast::Expr) -> Expr {
        let expr = self.expr(expr);
        let boolean = expr.truthy(&mut self.error_handler);
        debug_assert_eq!(boolean.ctype, Type::Bool);
        let zero = Expr::zero(boolean.location).implicit_cast(&Type::Bool, &mut self.error_handler);
        Expr {
            lval: false,
            location: boolean.location,
            ctype: Type::Int(true),
            expr: ExprType::Binary(
                BinaryOp::Compare(ComparisonToken::EqualEqual),
                Box::new(boolean),
                Box::new(zero),
            ),
        }
    }
    // a || b or a && b
    // NOTE: this short circuits if possible
    // 6.5.14 Logical OR operator and 6.5.13 Logical AND operator
    fn logical_bin_op(&mut self, a: Expr, b: Expr, op: BinaryOp) -> Expr {
        let a = a.implicit_cast(&Type::Bool, &mut self.error_handler);
        let b = b.implicit_cast(&Type::Bool, &mut self.error_handler);
        Expr {
            lval: false,
            // C logical operators yield int, while operands use internal truth values.
            ctype: Type::Int(true),
            location: a.location,
            expr: ExprType::Binary(op, Box::new(a), Box::new(b)),
        }
    }
    // condition ? then : otherwise
    // like an `if` in Rust: evaluate `condition`, yield the value of `then` if true, otherwise yield the value of `otherwise`
    fn ternary(
        &mut self,
        condition: ast::Expr,
        then: ast::Expr,
        otherwise: ast::Expr,
        location: Location,
    ) -> Expr {
        let condition = self.expr(condition).truthy(&mut self.error_handler);
        let mut then = self.expr(then).rval();
        let mut otherwise = self.expr(otherwise).rval();

        if then.ctype.is_arithmetic() && otherwise.ctype.is_arithmetic() {
            let (tmp1, tmp2) =
                Expr::binary_promote_for(then, otherwise, &mut self.error_handler, self.target);
            then = tmp1;
            otherwise = tmp2;
        } else if !pointer_promote(&mut then, &mut otherwise)
            // check that each part is not a type error
            && condition.ctype != Type::Error
            && then.ctype != Type::Error
            && otherwise.ctype != Type::Error
        {
            self.err(
                SemanticError::IncompatibleTypes(then.ctype.clone(), otherwise.ctype.clone()),
                location,
            );
        }
        Expr {
            ctype: then.ctype.clone(),
            lval: false,
            location,
            expr: ExprType::Ternary(Box::new(condition), Box::new(then), Box::new(otherwise)),
        }
    }

    // `a = b` or `a += b`
    fn assignment_expr(
        &mut self,
        lval: Expr,
        rval: Expr,
        token: lex::AssignmentToken,
        location: Location,
    ) -> Expr {
        if let Err(err) = lval.modifiable_lval() {
            self.err(err, location);
        }
        // `a = b`
        if let lex::AssignmentToken::Equal = token {
            let mut rval = rval.rval();
            if rval.ctype != lval.ctype {
                rval = rval.implicit_cast(&lval.ctype, &mut self.error_handler);
            }
            return Expr {
                ctype: lval.ctype.clone(),
                lval: false, // `(i = j) = 4`; is invalid
                location,
                expr: ExprType::Binary(BinaryOp::Assign, Box::new(lval), Box::new(rval)),
            };
        }
        if matches!(&lval.expr, ExprType::Id(_)) {
            let value = self
                .desugar_op(lval.clone().rval(), rval.rval(), token)
                .implicit_cast(&lval.ctype, &mut self.error_handler);
            return Expr {
                ctype: lval.ctype.clone(),
                lval: false,
                location,
                expr: ExprType::Binary(BinaryOp::Assign, Box::new(lval), Box::new(value)),
            };
        }
        // Complex assignment is tricky because the left side needs to be evaluated only once
        // Consider e.g. `*f() += 1`: `f()` should only be called once.
        // The hack implemented here is to treat `*f()` as a variable then load and store it to memory:
        // `tmp = &f(); *tmp = *tmp + 1;`
        // see also footnote 113 which has a similar algorithm (but is more convoluted because of atomics)

        // declare tmp in a new hidden scope
        // The declaration side channel carries storage only; evaluation stays
        // in the expression, including when nested in an initializer.
        self.scope.enter();
        let tmp_name = "tmp".into();
        let ctype = lval.ctype.clone();
        // TODO: we could probably make these qualifiers stronger
        let ptr_type = Type::Pointer(Box::new(ctype.clone()), Qualifiers::default());
        let meta = Variable {
            id: tmp_name,
            ctype: ptr_type.clone(),
            qualifiers: Qualifiers::NONE,
            storage_class: StorageClass::Register,
        };
        let tmp_var = self.declare(meta, true, location);

        // Preserve the original lvalue as the operand of its address capture.
        let address = Expr {
            ctype: ptr_type.clone(),
            lval: false,
            location,
            expr: ExprType::StaticRef(Box::new(lval)),
        };
        let temporary = Expr {
            ctype: ptr_type.clone(),
            lval: true,
            location,
            expr: ExprType::Id(tmp_var),
        };
        let set_address = Expr {
            ctype: ptr_type.clone(),
            lval: false,
            location,
            expr: ExprType::Binary(BinaryOp::Assign, Box::new(temporary), Box::new(address)),
        };
        let decl = Declaration {
            symbol: tmp_var,
            init: None,
        };
        self.decl_side_channel.push(Locatable::new(decl, location));
        self.scope.exit();
        // load `tmp`, i.e. `&*f()`, only evaluated once
        let tmp = Expr {
            expr: ExprType::Id(tmp_var),
            ctype: ptr_type,
            lval: true,
            location,
        }
        // this `rval` is because we have the (pointless) address of `tmp`
        // instead we want the address of the lval
        .rval();

        // before we had `&sum`, now we have `sum`
        // `*tmp`, i.e. `*f()`
        let lval_as_rval = Expr {
            ctype: ctype.clone(),
            lval: false,
            location,
            // this clone is pretty cheap since `tmp_assign_expr` is just an id
            expr: ExprType::Deref(Box::new(tmp.clone())),
        };
        // `*tmp` in an lval context
        let target = tmp.indirection(true, ctype.clone());
        // `*tmp + 1`
        let new_val = self
            .desugar_op(lval_as_rval, rval.rval(), token)
            .implicit_cast(&target.ctype, &mut self.error_handler);

        // Capture the address at expression evaluation time, including every
        // loop iteration and only the selected short-circuit branch.
        let update = Expr {
            ctype: ctype.clone(),
            lval: false,
            location,
            expr: ExprType::Binary(BinaryOp::Assign, Box::new(target), Box::new(new_val)),
        };
        Expr {
            ctype,
            lval: false,
            location,
            expr: ExprType::Comma(Box::new(set_address), Box::new(update)),
        }
    }
    fn desugar_op(&mut self, left: Expr, right: Expr, token: lex::AssignmentToken) -> Expr {
        use lex::AssignmentToken::*;

        match token {
            Equal => unreachable!(),
            OrEqual => self.parse_integer_op(left, right, BinaryOp::BitwiseOr),
            AndEqual => self.parse_integer_op(left, right, BinaryOp::BitwiseAnd),
            XorEqual => self.parse_integer_op(left, right, BinaryOp::Xor),
            ShlEqual => self.parse_integer_op(left, right, BinaryOp::Shl),
            ShrEqual => self.parse_integer_op(left, right, BinaryOp::Shr),
            MulEqual => self.mul(left, right, BinaryOp::Mul),
            DivEqual => self.mul(left, right, BinaryOp::Div),
            ModEqual => self.mul(left, right, BinaryOp::Mod),
            AddEqual => self.add(left, right, BinaryOp::Add),
            SubEqual => self.add(left, right, BinaryOp::Sub),
        }
    }
}

// literal
pub(super) fn literal(literal: LiteralValue, location: Location) -> Expr {
    literal_for(literal, location, crate::TargetDataModel::Sia32)
}

fn literal_for(literal: LiteralValue, location: Location, target: crate::TargetDataModel) -> Expr {
    use crate::data::types::ArrayType;

    let ctype = match &literal {
        LiteralValue::Char(_) => Type::Int(true),
        LiteralValue::Int(value)
            if target == crate::TargetDataModel::Amd64
                && *value >= i32::MIN as i64
                && *value <= i32::MAX as i64 =>
        {
            Type::Int(true)
        }
        LiteralValue::Int(value)
            if target == crate::TargetDataModel::Sia32
                && (*value > u32::MAX as i64 || *value < i32::MIN as i64) =>
        {
            Type::LongLong(true)
        }
        LiteralValue::Int(_) => Type::Long(true),
        LiteralValue::UnsignedInt(value)
            if target == crate::TargetDataModel::Amd64 && *value <= u32::MAX as u64 =>
        {
            Type::Int(false)
        }
        LiteralValue::UnsignedInt(value)
            if target == crate::TargetDataModel::Sia32 && *value > u32::MAX as u64 =>
        {
            Type::LongLong(false)
        }
        LiteralValue::UnsignedInt(_) => Type::Long(false),
        LiteralValue::Float(_) => Type::Double,
        LiteralValue::Str(s) => {
            let len = s.len() as arch::SIZE_T;
            Type::Array(Box::new(Type::Char(true)), ArrayType::Fixed(len))
        }
    };
    Expr {
        lval: false,
        ctype,
        location,
        expr: ExprType::Literal(literal),
    }
}

// Keep _Generic compatibility explicit: pointer pointee qualifiers are part
// of the type, arrays with unknown bounds can be compatible with fixed arrays,
// and function return types must agree even without a prototype.
fn generic_compatible(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Struct(a), Type::Struct(b)) | (Type::Union(a), Type::Union(b)) => match (a, b) {
            (StructType::Named(_, a), StructType::Named(_, b)) => a.same_identity(*b),
            (StructType::Anonymous(a), StructType::Anonymous(b)) => std::rc::Rc::ptr_eq(a, b),
            _ => false,
        },
        (Type::Pointer(a, aq), Type::Pointer(b, bq)) => aq == bq && generic_compatible(a, b),
        (Type::Array(a, al), Type::Array(b, bl)) => {
            generic_compatible(a, b)
                && match (al, bl) {
                    (ArrayType::Fixed(a), ArrayType::Fixed(b)) => a == b,
                    _ => true,
                }
        }
        (Type::Function(a), Type::Function(b)) => {
            if !generic_compatible(&a.return_type, &b.return_type) {
                return false;
            }
            if a.params.is_empty() || b.params.is_empty() {
                let proto = if a.params.is_empty() { b } else { a };
                return !proto.varargs
                    && proto.params.iter().all(|p| {
                        matches!(
                            p.get().ctype,
                            Type::Void
                                | Type::Int(_)
                                | Type::Long(_)
                                | Type::LongLong(_)
                                | Type::Double
                                | Type::LongDouble
                                | Type::Pointer(_, _)
                                | Type::Struct(_)
                                | Type::Union(_)
                        )
                    });
            }
            a.varargs == b.varargs
                && a.params.len() == b.params.len()
                && a.params
                    .iter()
                    .zip(&b.params)
                    .all(|(a, b)| generic_compatible(&a.get().ctype, &b.get().ctype))
        }
        // Cosmic's enum representation is a signed int (distinct enum tags
        // remain incompatible with one another).
        (Type::Enum(_, _), Type::Int(true)) | (Type::Int(true), Type::Enum(_, _)) => true,
        _ => a == b,
    }
}
fn generic_complete_object(ty: &Type) -> bool {
    match ty {
        Type::Void | Type::Function(_) | Type::Error => false,
        Type::Array(element, ArrayType::Fixed(_)) => generic_complete_object(element),
        Type::Array(_, _) => false,
        Type::Struct(st) | Type::Union(st) => !st.is_empty(),
        _ => true,
    }
}
fn generic_variably_modified(ty: &Type) -> bool {
    match ty {
        Type::Array(_, ArrayType::Variable(_)) => true,
        Type::Pointer(inner, _) | Type::Array(inner, _) => generic_variably_modified(inner),
        Type::Function(f) => {
            generic_variably_modified(&f.return_type)
                || f.params
                    .iter()
                    .any(|p| generic_variably_modified(&p.get().ctype))
        }
        _ => false,
    }
}

// 6.5.15 - Conditional operator
fn compatible_pointer_types(left: &Type, right: &Type) -> bool {
    match (left, right) {
        // Ignore only immediate pointee qualifiers. Nested pointer qualifiers
        // remain part of the type and must not acquire unsafe conversions.
        (Type::Pointer(a, _), Type::Pointer(b, _)) => a == b,
        _ => false,
    }
}

fn pointer_promote(left: &mut Expr, right: &mut Expr) -> bool {
    if left.ctype == right.ctype {
        return true;
    }
    let common = match (&left.ctype, &right.ctype) {
        (Type::Pointer(a, aq), Type::Pointer(b, bq)) => {
            let pointee = if a == b {
                a.clone()
            } else if (a.as_ref() == &Type::Void && !b.is_function())
                || (b.as_ref() == &Type::Void && !a.is_function())
            {
                Box::new(Type::Void)
            } else {
                return false;
            };
            Type::Pointer(
                pointee,
                Qualifiers {
                    c_const: aq.c_const || bq.c_const,
                    volatile: aq.volatile || bq.volatile,
                    func: aq.func,
                },
            )
        }
        (_, Type::Pointer(_, _)) if left.is_null() => right.ctype.clone(),
        (Type::Pointer(_, _), _) if right.is_null() => left.ctype.clone(),
        _ => return false,
    };
    left.ctype = common.clone();
    right.ctype = common;
    true
}

impl Type {
    #[inline]
    fn is_void_pointer(&self) -> bool {
        match self {
            Type::Pointer(t, _) => **t == Type::Void,
            _ => false,
        }
    }
    #[inline]
    /// used for pointer addition and subtraction, see section 6.5.6 of the C11 standard
    fn is_pointer_to_complete_object(&self) -> bool {
        match self {
            Type::Pointer(ctype, _) => ctype.is_complete() && !ctype.is_function(),
            Type::Array(_, _) => true,
            _ => false,
        }
    }
    /// Return whether self is a signed type.
    ///
    /// Should only be called on integral types.
    /// Calling sign() on a floating or derived type will return Err(()).
    fn sign(&self) -> Result<bool, ()> {
        use Type::*;
        match self {
            SignedChar => Ok(true),
            Char(sign) | Short(sign) | Int(sign) | Long(sign) | LongLong(sign) => Ok(*sign),
            Bool => Ok(false),
            // TODO: allow enums with values of UINT_MAX
            Enum(_, _) => Ok(true),
            _ => Err(()),
        }
    }

    /// Return the rank of an integral type, according to section 6.3.1.1 of the C standard.
    ///
    /// It is an error to take the rank of a non-integral type.
    ///
    /// Examples:
    /// ```ignore
    /// use saltwater::data::types::Type::*;
    /// assert!(Long(true).rank() > Int(true).rank());
    /// assert!(Int(false).rank() > Short(false).rank());
    /// assert!(Short(true).rank() > Char(true).rank());
    /// assert!(Char(true).rank() > Bool.rank());
    /// assert!(Long(false).rank() > Bool.rank());
    /// assert!(Long(true).rank() == Long(false).rank());
    /// ```
    fn rank(&self) -> usize {
        use Type::*;
        match self {
            Bool => 0,
            Char(_) | SignedChar => 1,
            Short(_) => 2,
            Int(_) => 3,
            Long(_) => 4,
            LongLong(_) => 5,
            _ => usize::MAX,
        }
    }
    // Subclause 2 of 6.3.1.1 Boolean, characters, and integers
    fn integer_promote(self) -> Type {
        if self.rank() <= Type::Int(true).rank() {
            if Type::Int(true).can_represent(&self) {
                Type::Int(true)
            } else {
                Type::Int(false)
            }
        } else {
            self
        }
    }
    // 6.3.1.8 Usual arithmetic conversions
    fn binary_promote_for(
        mut left: Type,
        mut right: Type,
        target: crate::TargetDataModel,
    ) -> Result<Type, Type> {
        use Type::*;
        if left == LongDouble || right == LongDouble {
            return Ok(LongDouble);
        }
        if left == Double || right == Double {
            return Ok(Double); // toil and trouble
        } else if left == Float || right == Float {
            return Ok(Float);
        }
        left = left.integer_promote();
        right = right.integer_promote();
        // TODO: we know that `left` can't be used after a move,
        // but rustc isn't smart enough to figure it out and let us remove the clone
        let signs = (
            left.sign().map_err(|_| left.clone())?,
            right.sign().map_err(|_| right.clone())?,
        );
        // same sign
        if signs.0 == signs.1 {
            return Ok(if left.rank() >= right.rank() {
                left
            } else {
                right
            });
        };
        let (signed, unsigned) = if signs.0 {
            (left, right)
        } else {
            (right, left)
        };
        if signed.can_represent_for(&unsigned, target) {
            Ok(signed)
        } else {
            Ok(unsigned)
        }
    }
    /// 6.5.2.2p6:
    /// > If the expression that denotes the called function has a type that does not include a prototype,
    /// > the integer promotions are performed on each argument,
    /// > and arguments that have type float are promoted to double.
    /// > These are called the default argument promotions.
    fn default_promote(self) -> Type {
        if self.is_integral() {
            self.integer_promote()
        } else if self == Type::Float {
            Type::Double
        } else {
            self
        }
    }
    fn is_struct(&self) -> bool {
        match self {
            Type::Struct(_) | Type::Union(_) => true,
            _ => false,
        }
    }
    fn is_complete(&self) -> bool {
        match self {
            Type::Void | Type::Function(_) | Type::Array(_, types::ArrayType::Unbounded) => false,
            // TODO: update when we allow incomplete struct and union types (e.g. `struct s;`)
            _ => true,
        }
    }
}

impl Expr {
    pub(super) fn zero(location: Location) -> Expr {
        Expr {
            ctype: Type::Int(true),
            expr: ExprType::Literal(LiteralValue::Int(0)),
            lval: false,
            location,
        }
    }
    // 6.3.2.3 Pointers
    fn is_null(&self) -> bool {
        // TODO: I think we need to const fold this to allow `(void*)0`
        if let ExprType::Literal(token) = &self.expr {
            match token {
                LiteralValue::Int(0) | LiteralValue::UnsignedInt(0) | LiteralValue::Char(0) => true,
                _ => false,
            }
        } else {
            false
        }
    }
    fn id(symbol: Symbol, location: Location) -> Self {
        Self {
            expr: ExprType::Id(symbol),
            // TODO: maybe pass in the type as well to avoid the lookup?
            // but then we need to make sure the type matches the symbol
            ctype: symbol.get().ctype.clone(),
            lval: true,
            location,
        }
    }
    // Convert an expression to _Bool. Section 6.3.1.3 of the C standard:
    // "When any scalar value is converted to _Bool,
    // the result is 0 if the value compares equal to 0; otherwise, the result is 1."
    //
    // TODO: this looks like the same as casting to _Bool, can we just offload to the backend instead?
    //
    // if (expr)
    pub(crate) fn truthy(mut self, error_handler: &mut ErrorHandler) -> Expr {
        self = self.rval();
        if self.ctype == Type::Bool {
            return self;
        }
        // if not scalar and is not a type error
        if !self.ctype.is_scalar() && self.ctype != Type::Error {
            error_handler.error(
                SemanticError::Generic(format!(
                    "expression of type '{}' cannot be converted to bool",
                    self.ctype
                )),
                self.location,
            );
            self.ctype = Type::Error;
        }
        let zero = Expr::zero(self.location).implicit_cast(&self.ctype, error_handler);
        Expr {
            lval: false,
            location: self.location,
            ctype: Type::Bool,
            expr: ExprType::Binary(
                BinaryOp::Compare(ComparisonToken::NotEqual),
                Box::new(self),
                Box::new(zero),
            ),
        }
    }

    // Perform an integer conversion, including all relevant casts.
    //
    // See `Type::integer_promote` for conversion rules.
    fn integer_promote(self, error_handler: &mut ErrorHandler) -> Expr {
        let expr = self.rval();
        let ctype = expr.ctype.clone().integer_promote();
        expr.implicit_cast(&ctype, error_handler)
    }

    // Perform a binary conversion, including all relevant casts.
    //
    // See `Type::binary_promote` for conversion rules.
    fn binary_promote_for(
        left: Expr,
        right: Expr,
        error_handler: &mut ErrorHandler,
        target: crate::TargetDataModel,
    ) -> (Expr, Expr) {
        let (left, right) = (left.rval(), right.rval());
        let ctype = Type::binary_promote_for(left.ctype.clone(), right.ctype.clone(), target);
        match ctype {
            Ok(promoted) => (
                left.implicit_cast(&promoted, error_handler),
                right.implicit_cast(&promoted, error_handler),
            ),
            Err(non_int) => {
                // TODO: this location is wrong
                if left.ctype != Type::Error && right.ctype != Type::Error {
                    error_handler.error(SemanticError::NonIntegralExpr(non_int), right.location);
                }
                (left, right)
            }
        }
    }
    // ensure an expression has a value. convert
    // - arrays -> pointers
    // - functions -> pointers
    // - variables -> value stored in that variable
    // 6.3.2.1 Lvalues, arrays, and function designators
    // >  Except when it is the operand of [a bunch of different operators],
    // > an lvalue that does not have array type is converted to the value stored in the designated object (and is no longer an lvalue)
    pub(super) fn rval(self) -> Expr {
        let qualifiers = self.lvalue_qualifiers();
        match self.ctype {
            // a + 1 is the same as &a + 1
            Type::Array(to, _) => Expr {
                lval: false,
                ctype: Type::Pointer(to, qualifiers),
                ..self
            },
            Type::Function(_) => Expr {
                lval: false,
                ctype: Type::Pointer(Box::new(self.ctype), Qualifiers::NONE),
                ..self
            },
            // HACK: structs can't be dereferenced since they're not scalar, so we just fake it
            Type::Struct(_) | Type::Union(_) if self.lval => Expr {
                lval: false,
                ..self
            },
            _ if self.lval => Expr {
                ctype: self.ctype.clone(),
                lval: false,
                location: self.location,
                expr: ExprType::Deref(Box::new(self)),
            },
            _ => self,
        }
    }
    // `*p` when p is a pointer
    //
    // NOTE: this can be in _either_ an lval or an rval context
    // in an rval context: `return *p;`
    // in an lval context: `*p = 1`
    //
    // `ctype` is the type of the resulting expression
    //
    // 6.5.3.2 Address and indirection operators
    fn indirection(self, lval: bool, ctype: Type) -> Self {
        Expr {
            location: self.location,
            ctype,
            lval,
            // this is super hacky but the only way I can think of to prevent
            // https://github.com/jyn514/rcc/issues/90
            // we need to call `self.rval()` so that if `self` is a variable we get its value, not its address.
            expr: ExprType::Noop(Box::new(self.rval())),
        }
    }

    // float f = (double)1.0
    // 6.3 Conversions
    pub(super) fn implicit_cast(self, ctype: &Type, error_handler: &mut ErrorHandler) -> Expr {
        let mut expr = self.rval();
        if &expr.ctype == ctype {
            expr
        // int -> long
        } else if expr.ctype.is_arithmetic() && ctype.is_arithmetic()
            // NULL -> int*
            || expr.is_null() && ctype.is_pointer()
            // if ((int*)p)
            || expr.ctype.is_pointer() && ctype.is_bool()
        {
            Expr {
                location: expr.location,
                expr: ExprType::Cast(Box::new(expr)),
                lval: false,
                ctype: ctype.clone(),
            }
        } else if expr.ctype == Type::Error {
            expr
        } else {
            // allow implicit casts of const pointers
            // Standard (in the context `left = right`, i.e. casting `right` to `left`)
            // > the left operand has atomic, qualified, or unqualified pointer type,
            // > and (considering the type the left operand would have after lvalue conversion)
            // > both operands are pointers to qualified or unqualified versions of compatible types,
            // > and the type pointed to by the left has all the qualifiers of the type pointed to by the right;
            if let (Type::Pointer(a, from), Type::Pointer(b, to)) = (&expr.ctype, ctype) {
                let compatible = a == b
                    || (matches!(a.as_ref(), Type::Void) && !b.is_function())
                    || (matches!(b.as_ref(), Type::Void) && !a.is_function());
                if compatible && to.contains_all(*from) {
                    expr.ctype = ctype.clone();
                    return expr;
                }
            }
            // There is probably a better way to do this
            // don't report cascading errors
            if *ctype != Type::Error {
                error_handler.error(
                    SemanticError::InvalidCast(expr.ctype.clone(), ctype.clone()),
                    expr.location,
                );
            }
            expr
        }
    }
    fn lvalue_qualifiers(&self) -> Qualifiers {
        match &self.expr {
            ExprType::Id(symbol) => symbol.get().qualifiers,
            ExprType::Noop(pointer)
                if self.lval || matches!(self.ctype, Type::Struct(_) | Type::Union(_)) =>
            {
                match &pointer.ctype {
                    Type::Pointer(_, qualifiers) => *qualifiers,
                    _ => Qualifiers::NONE,
                }
            }
            ExprType::Member(compound, id) => {
                let inherited = compound.lvalue_qualifiers();
                let member = match &compound.ctype {
                    Type::Struct(stype) | Type::Union(stype) => stype
                        .members()
                        .iter()
                        .find(|member| member.id == *id)
                        .map(|member| member.qualifiers)
                        .unwrap_or(Qualifiers::NONE),
                    _ => Qualifiers::NONE,
                };
                Qualifiers {
                    c_const: inherited.c_const || member.c_const,
                    volatile: inherited.volatile || member.volatile,
                    ..Qualifiers::NONE
                }
            }
            _ => Qualifiers::NONE,
        }
    }

    /// See section 6.3.2.1 of the C Standard. In particular:
    /// "A modifiable lvalue is an lvalue that does not have array type,
    /// does not  have an incomplete type, does not have a const-qualified type,
    /// and if it is a structure or union, does not have any member with a const-qualified type"
    fn modifiable_lval(&self) -> Result<(), SemanticError> {
        let err = |e| Err(SemanticError::NotAssignable(e));
        // rval
        if !self.lval {
            return err("rvalue".to_string());
        }
        // incomplete type
        if !self.ctype.is_complete() {
            return err(format!("expression with incomplete type '{}'", self.ctype));
        }
        if self.lvalue_qualifiers().c_const {
            return err("object with `const` qualifier".to_string());
        }
        match &self.ctype {
            // array type
            Type::Array(_, _) => err("array".to_string()),
            // member with const-qualified type
            Type::Struct(stype) | Type::Union(stype) => {
                if stype
                    .members()
                    .iter()
                    .map(|sym| sym.qualifiers.c_const)
                    .any(|x| x)
                {
                    err("struct or union with `const` qualified member".to_string())
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

impl Qualifiers {
    // return whether `self` has all the qualifiers of `right`
    // WARNING: this _must_ be updated if you add more fields to `Qualifiers`
    fn contains_all(self, other: Self) -> bool {
        (self.c_const || !other.c_const) && (self.volatile || !other.volatile)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::analyze::test::analyze;
    use crate::analyze::*;
    pub(crate) fn expr(input: &str) -> CompileResult<Expr> {
        analyze(input, Parser::expr, PureAnalyzer::expr)
    }
    fn get_location(r: &CompileResult<Expr>) -> Location {
        match r {
            Ok(expr) => expr.location,
            Err(err) => err.location(),
        }
    }
    fn assert_literal(token: LiteralValue) {
        let parsed = expr(&token.to_string());
        let location = get_location(&parsed);
        assert_eq!(parsed.unwrap(), literal(token, location));
    }
    fn expr_with_scope<'a>(input: &'a str, variables: &[Symbol]) -> CompileResult<Expr> {
        analyze(input, Parser::expr, |a, expr| {
            for &meta in variables {
                let id = meta.get().id;
                a.scope.insert(id, meta);
            }
            a.expr(expr)
        })
    }
    fn assert_type(input: &str, ctype: Type) {
        match expr(input) {
            Ok(expr) => assert_eq!(expr.ctype, ctype),
            Err(err) => panic!("error: {}", err.data),
        };
    }
    #[test]
    fn test_primaries() {
        assert_literal(LiteralValue::Int(141));
        let parsed = expr("\"hi there\"");

        assert_eq!(
            parsed,
            Ok(literal(
                LiteralValue::Str("hi there\0".into()),
                get_location(&parsed)
            )),
        );
        assert_literal(LiteralValue::Float(1.5));
        let parsed = expr("(1)");
        assert_eq!(
            parsed,
            Ok(literal(LiteralValue::Int(1), get_location(&parsed)))
        );
        let x = Variable {
            ctype: Type::Int(true),
            id: InternedStr::get_or_intern("x"),
            qualifiers: Default::default(),
            storage_class: Default::default(),
        }
        .insert();
        let parsed = expr_with_scope("x", &[x]);
        assert_eq!(
            parsed,
            Ok(Expr {
                location: get_location(&parsed),
                ctype: Type::Int(true),
                lval: true,
                expr: ExprType::Id(x)
            })
        );
    }
    #[test]
    fn qualified_pointer_expression_compatibility() {
        let int = Type::Int(true);
        let pointer = |q| Type::Pointer(Box::new(int.clone()), q);
        let const_q = Qualifiers {
            c_const: true,
            ..Qualifiers::NONE
        };
        let volatile_q = Qualifiers {
            volatile: true,
            ..Qualifiers::NONE
        };
        let variables: Vec<_> = [
            ("p", pointer(Qualifiers::NONE)),
            ("cp", pointer(const_q)),
            ("vp", pointer(volatile_q)),
            (
                "arr",
                Type::Array(Box::new(int.clone()), types::ArrayType::Fixed(8)),
            ),
            ("v", Type::Pointer(Box::new(Type::Void), Qualifiers::NONE)),
            (
                "ch",
                Type::Pointer(Box::new(Type::Char(true)), Qualifiers::NONE),
            ),
        ]
        .iter()
        .map(|(id, ctype)| {
            Variable {
                id: InternedStr::get_or_intern(*id),
                ctype: ctype.clone(),
                qualifiers: Qualifiers::NONE,
                storage_class: Default::default(),
            }
            .insert()
        })
        .collect();
        for text in ["p < cp", "cp >= p", "vp == cp"] {
            assert_eq!(
                expr_with_scope(text, &variables).unwrap().ctype,
                Type::Int(true)
            );
        }
        for text in ["cp - p", "cp - arr", "arr - cp"] {
            assert_eq!(
                expr_with_scope(text, &variables).unwrap().ctype,
                Type::Long(true)
            );
        }
        let common = pointer(Qualifiers {
            c_const: true,
            volatile: true,
            ..Qualifiers::NONE
        });
        assert_eq!(
            expr_with_scope("1 ? cp : vp", &variables).unwrap().ctype,
            common
        );
        assert_eq!(
            expr_with_scope("1 ? 0 : cp", &variables).unwrap().ctype,
            pointer(const_q)
        );
        assert_eq!(
            expr_with_scope("1 ? cp : v", &variables).unwrap().ctype,
            Type::Pointer(Box::new(Type::Void), const_q)
        );
        for text in ["1 ? ch : p", "p + cp", "p < v"] {
            assert!(expr_with_scope(text, &variables).is_err(), "{}", text);
        }
        for text in ["++p", "--p", "*--p", "p += 2", "p -= 2"] {
            assert!(expr_with_scope(text, &variables).is_ok(), "{}", text);
        }
        assert!(expr_with_scope("p = (1 ? cp : p)", &variables).is_err());
        let subtract = expr_with_scope("p - 3", &variables).unwrap();
        assert!(matches!(
            subtract.expr,
            ExprType::Binary(BinaryOp::Sub, _, _)
        ));
    }

    #[test]
    fn test_mul() {
        assert_type("1*1.0", Type::Double);
        assert_type("1*2.0 / 1.3", Type::Double);
        assert_type("3%2", Type::Long(true));
    }
    #[test]
    fn test_funcall() {
        let f = Variable {
            id: InternedStr::get_or_intern("f"),
            qualifiers: Default::default(),
            storage_class: Default::default(),
            ctype: Type::Function(types::FunctionType {
                params: vec![Variable {
                    ctype: Type::Void,
                    id: Default::default(),
                    qualifiers: Default::default(),
                    storage_class: StorageClass::Auto,
                }
                .insert()],
                return_type: Box::new(Type::Int(true)),
                varargs: false,
            }),
        }
        .insert();
        assert!(expr_with_scope("f(1,2,3)", &[f]).is_err());
        let parsed = expr_with_scope("f()", &[f]);
        assert!(match parsed {
            Ok(Expr {
                expr: ExprType::FuncCall(_, _),
                ..
            }) => true,
            _ => false,
        },);
    }
    #[test]
    fn test_type_errors() {
        assert!(expr("1 % 2.0").is_err());
        assert!(expr("0 ? \"error message\" : 0.0").is_err());
    }

    #[test]
    fn test_explicit_casts() {
        assert_type("(int)4.2", Type::Int(true));
        assert_type("(unsigned int)4.2", Type::Int(false));
        assert_type("(float)4.2", Type::Float);
        assert_type("(double)4.2", Type::Double);
        assert!(expr("(int*)4.2").is_err());
        assert_type(
            "(int*)(int)4.2",
            Type::Pointer(Box::new(Type::Int(true)), Qualifiers::default()),
        );
    }
    #[test]
    fn long_long_width_rank_and_suffixes_are_preserved() {
        assert_eq!(Type::LongLong(true).sizeof().unwrap(), 8);
        assert_eq!(Type::LongLong(false).alignof().unwrap(), 8);
        assert!(Type::LongLong(true).rank() > Type::Long(false).rank());
        assert_type("1LL", Type::LongLong(true));
        assert_type("1ULL", Type::LongLong(false));
        assert_type("1LLU", Type::LongLong(false));
        assert_type("4294967296", Type::LongLong(true));
        assert_type("18446744073709551615ULL", Type::LongLong(false));
        assert_type("(long long)1 + (unsigned)2", Type::LongLong(true));
        assert_type(
            "(unsigned long long)1 + (long long)2",
            Type::LongLong(false),
        );
    }

    #[test]
    fn narrow_pointer_indices_scale_without_truncation() {
        for target in [crate::TargetDataModel::Sia32, crate::TargetDataModel::Amd64] {
            let mut analyzer = PureAnalyzer::new_for_target(target);
            let location = Location::default();
            let base = literal(LiteralValue::UnsignedInt(0), location);
            let index = literal(LiteralValue::UnsignedInt(255), location)
                .implicit_cast(&Type::Char(false), &mut analyzer.error_handler);
            let indexed = analyzer.pointer_arithmetic(base, index, &Type::Int(true), location);
            let offset = match indexed.expr {
                ExprType::Binary(BinaryOp::Add, _, offset) => offset,
                _ => panic!("scaled pointer addition"),
            };
            assert_eq!(offset.ctype, Type::Long(true));
            let folded = offset.const_fold_for(target).unwrap();
            assert!(matches!(
                folded.expr,
                ExprType::Literal(LiteralValue::Int(1020))
            ));
        }
    }
}
