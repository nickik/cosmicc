//! Conservative storage escape analysis before frontend Variable SSA lowering.
use super::*;
use saltwater_parser::data::hir::{BinaryOp, StmtType};
use std::collections::HashSet;
#[derive(Default)]
struct Analysis {
    candidates: Vec<Symbol>,
    escaped: HashSet<Symbol>,
    referenced: HashSet<Symbol>,
}
impl Analysis {
    fn candidate(&mut self, s: Symbol) {
        let m = s.get();
        if matches!(m.storage_class, StorageClass::Auto | StorageClass::Register)
            && !m.qualifiers.volatile
            && !address_type(&m.ctype)
            && ir_type(&m.ctype).is_ok()
            && !self.candidates.contains(&s)
        {
            self.candidates.push(s);
        }
    }
    fn expr(&mut self, e: &Expr) {
        if let ExprType::Id(s) = e.expr {
            self.referenced.insert(s);
        }
        match &e.expr {
            ExprType::Id(s) => {
                self.escaped.insert(*s);
            }
            // Scalar reads and writes use the object value without exposing storage.
            ExprType::Deref(v) | ExprType::PostIncrement(v, _)
                if matches!(v.expr, ExprType::Id(_)) =>
            {
                if let ExprType::Id(s) = v.expr {
                    self.referenced.insert(s);
                }
            }
            ExprType::Binary(BinaryOp::Assign, a, b) => {
                if !matches!(a.expr, ExprType::Id(_)) {
                    self.expr(a);
                }
                self.expr(b);
            }
            ExprType::FuncCall(a, args) => {
                self.expr(a);
                for e in args {
                    self.expr(e);
                }
            }
            ExprType::Member(a, _)
            | ExprType::Cast(a)
            | ExprType::Deref(a)
            | ExprType::Negate(a)
            | ExprType::BitwiseNot(a)
            | ExprType::StaticRef(a)
            | ExprType::Noop(a)
            | ExprType::PostIncrement(a, _) => self.expr(a),
            ExprType::Binary(_, a, b) | ExprType::Comma(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprType::Ternary(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            ExprType::Sizeof(_) | ExprType::Literal(_) => {}
        }
    }
    fn init(&mut self, i: &Initializer) {
        match i {
            Initializer::Scalar(e) => self.expr(e),
            Initializer::InitializerList(v) => {
                for i in v {
                    self.init(i);
                }
            }
            _ => {}
        }
    }
    fn stmt(&mut self, s: &Stmt) {
        match &s.data {
            StmtType::Compound(v) => {
                for s in v {
                    self.stmt(s);
                }
            }
            StmtType::If(e, a, b) => {
                self.expr(e);
                self.stmt(a);
                if let Some(b) = b {
                    self.stmt(b);
                }
            }
            StmtType::Do(a, e) | StmtType::While(e, a) | StmtType::Switch(e, a) => {
                self.expr(e);
                self.stmt(a);
            }
            StmtType::For(a, b, c, d) => {
                self.stmt(a);
                if let Some(e) = b {
                    self.expr(e);
                }
                if let Some(e) = c {
                    self.expr(e);
                }
                self.stmt(d);
            }
            StmtType::Label(_, s) | StmtType::Case(_, s) | StmtType::Default(s) => self.stmt(s),
            StmtType::Expr(e) | StmtType::Return(Some(e)) => self.expr(e),
            StmtType::Decl(v) => {
                for d in v {
                    self.candidate(d.data.symbol);
                    if let Some(i) = &d.data.init {
                        self.init(i);
                    }
                }
            }
            _ => {}
        }
    }
}
pub(super) fn variables(
    b: &mut FunctionBuilder<'_>,
    ft: &FunctionType,
    body: &[Stmt],
) -> HashMap<Symbol, cranelift_frontend::Variable> {
    let mut a = Analysis::default();
    for s in params(ft) {
        a.candidate(*s);
    }
    for s in body {
        a.stmt(s);
    }
    a.candidates
        .into_iter()
        .filter(|s| !a.escaped.contains(s))
        .map(|s| (s, b.declare_var(ir_type(&s.get().ctype).unwrap())))
        .collect()
}

pub(super) fn referenced(body: &[Stmt]) -> HashSet<Symbol> {
    let mut analysis = Analysis::default();
    for statement in body {
        analysis.stmt(statement);
    }
    analysis.referenced
}
