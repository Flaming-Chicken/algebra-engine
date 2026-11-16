//! Arena-allocated DAG storage with lock-free hash-consing deduplication.

use crate::domain::Domain;
use crate::expr::{ExprKind, ExprNode, RelOp};
use crate::id::{ExprId, SymbolId};
use crate::number::{Constant, Number};
use crate::symbol::SymbolTable;
use dashmap::DashMap;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::sync::Arc;

/// Arena-allocated, thread-safe Expression DAG graph.
///
/// Ensures structural deduplication (Hash-Consing) so that identical nodes
/// share a single [`ExprId`] in memory.
#[derive(Debug, Clone)]
pub struct ExprGraph {
    nodes: Arc<RwLock<Vec<ExprNode>>>,
    dedup_cache: Arc<DashMap<ExprNode, ExprId>>,
    pub symbols: SymbolTable,
}

impl Default for ExprGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ExprGraph {
    /// Create a new empty expression graph arena.
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
            dedup_cache: Arc::new(DashMap::new()),
            symbols: SymbolTable::new(),
        }
    }

    /// Intern an expression node into the Arena DAG.
    ///
    /// If an identical node already exists, returns the existing [`ExprId`].
    /// Otherwise, inserts the node into the Arena and returns its new [`ExprId`].
    pub fn intern(&self, node: ExprNode) -> ExprId {
        if let Some(id) = self.dedup_cache.get(&node) {
            tracing::trace!(node_kind = ?node.kind, "Hash-consing cache hit");
            return *id;
        }

        let mut nodes_guard = self.nodes.write();
        if let Some(id) = self.dedup_cache.get(&node) {
            return *id;
        }

        let new_id = ExprId::new(nodes_guard.len() as u32);
        nodes_guard.push(node.clone());
        self.dedup_cache.insert(node.clone(), new_id);
        tracing::debug!(node_id = ?new_id, node_kind = ?node.kind, "Interned new ExprNode in DAG");

        new_id
    }

    /// Intern a batch of expression nodes into the Arena DAG in a single lock operation.
    ///
    /// Significantly reduces lock contention when inserting multiple nodes created concurrently.
    pub fn intern_batch<I: IntoIterator<Item = ExprNode>>(&self, batch: I) -> Vec<ExprId> {
        let nodes_vec: Vec<ExprNode> = batch.into_iter().collect();
        let mut results = Vec::with_capacity(nodes_vec.len());
        let mut to_insert = Vec::new();

        for node in nodes_vec {
            if let Some(id) = self.dedup_cache.get(&node) {
                results.push(*id);
            } else {
                to_insert.push(node);
                // Placeholder ID position
                results.push(ExprId::new(u32::MAX));
            }
        }

        if to_insert.is_empty() {
            return results;
        }

        let mut nodes_guard = self.nodes.write();
        let mut insert_idx = 0;
        for res in &mut results {
            if res.index() == u32::MAX {
                let node = &to_insert[insert_idx];
                insert_idx += 1;
                if let Some(id) = self.dedup_cache.get(node) {
                    *res = *id;
                } else {
                    let new_id = ExprId::new(nodes_guard.len() as u32);
                    nodes_guard.push(node.clone());
                    self.dedup_cache.insert(node.clone(), new_id);
                    *res = new_id;
                }
            }
        }

        results
    }

    /// Retrieve a cloned copy of the [`ExprNode`] at the given [`ExprId`].
    pub fn get(&self, id: ExprId) -> ExprNode {
        let guard = self.nodes.read();
        guard[id.as_usize()].clone()
    }

    /// Get the total count of unique allocated nodes in the arena.
    pub fn len(&self) -> usize {
        self.nodes.read().len()
    }

    /// Returns `true` if the expression graph contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // --- High-level node constructor helpers ---

    /// Intern a symbol variable with default Real domain.
    pub fn symbol(&self, name: &str) -> ExprId {
        self.symbol_with_domain(name, Domain::Reals)
    }

    /// Intern a symbol variable with a specified domain.
    pub fn symbol_with_domain(&self, name: &str, domain: Domain) -> ExprId {
        let sym_id = self.symbols.get_or_intern(name);
        self.intern(ExprNode::new(domain, ExprKind::Symbol(sym_id)))
    }

    /// Intern a scalar Number directly, inferring the appropriate mathematical domain.
    pub fn number(&self, n: Number) -> ExprId {
        let domain = match &n {
            Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
            Number::Rational(_, _) | Number::BigRational(_) | Number::Scientific { .. } => {
                Domain::Rationals
            }
            Number::Constant(Constant::I) => Domain::Complex,
            _ => Domain::Reals,
        };
        self.intern(ExprNode::new(domain, ExprKind::Number(n)))
    }

    /// Intern an integer scalar.
    pub fn integer(&self, val: i64) -> ExprId {
        self.intern(ExprNode::new(
            Domain::Integers,
            ExprKind::Number(Number::Integer(val)),
        ))
    }

    /// Intern a rational fraction scalar in reduced form.
    pub fn rational(&self, num: i64, den: i64) -> ExprId {
        let n = Number::rational(num, den);
        let domain = match &n {
            Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
            _ => Domain::Rationals,
        };
        self.intern(ExprNode::new(domain, ExprKind::Number(n)))
    }

    /// Intern an exact scientific notation scalar: mantissa * 10^exponent.
    pub fn scientific(&self, mantissa: i64, exponent: i32) -> ExprId {
        let n = Number::scientific(mantissa, exponent);
        let domain = match &n {
            Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
            _ => Domain::Rationals,
        };
        self.intern(ExprNode::new(domain, ExprKind::Number(n)))
    }

    /// Intern a float scalar, decomposing into a lossless exact representation by default.
    pub fn float(&self, val: f64) -> ExprId {
        let n = Number::from_f64_lossless(val);
        let domain = match &n {
            Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
            Number::Rational(_, _) | Number::BigRational(_) | Number::Scientific { .. } => {
                Domain::Rationals
            }
            _ => Domain::Reals,
        };
        self.intern(ExprNode::new(domain, ExprKind::Number(n)))
    }

    /// Intern an explicit lossy float scalar preserving raw IEEE-754 bits.
    pub fn float_lossy(&self, val: f64) -> ExprId {
        self.intern(ExprNode::new(
            Domain::Reals,
            ExprKind::Number(Number::float(val)),
        ))
    }

    /// Intern a mathematical constant (e.g. π, e, i).
    pub fn constant(&self, c: Constant) -> ExprId {
        let domain = match c {
            Constant::I => Domain::Complex,
            _ => Domain::Reals,
        };
        self.intern(ExprNode::new(domain, ExprKind::Number(Number::Constant(c))))
    }

    /// Add multiple term expressions together with zero-term folding and lossless constant folding.
    pub fn add<I: IntoIterator<Item = ExprId>>(&self, terms: I) -> ExprId {
        let vec: SmallVec<[ExprId; 4]> = terms.into_iter().collect();
        let mut non_zero: SmallVec<[ExprId; 4]> = SmallVec::new();
        let mut const_acc: Option<Number> = None;

        for &t in &vec {
            let node = self.get(t);
            if let ExprKind::Number(n) = &node.kind {
                if n.is_zero() {
                    continue;
                }
                if n.is_lossless() {
                    if let Some(acc) = &const_acc {
                        if let Some(sum) = acc.add_lossless(n) {
                            const_acc = Some(sum);
                            continue;
                        }
                    } else {
                        const_acc = Some(n.clone());
                        continue;
                    }
                }
            }
            non_zero.push(t);
        }

        if let Some(acc) = const_acc
            && !acc.is_zero()
        {
            let domain = match &acc {
                Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
                _ => Domain::Rationals,
            };
            let acc_id = self.intern(ExprNode::new(domain, ExprKind::Number(acc)));
            non_zero.push(acc_id);
        }

        if non_zero.is_empty() {
            return self.integer(0);
        }
        if non_zero.len() == 1 {
            return non_zero[0];
        }
        let domain = self.infer_poly_domain(&non_zero);
        self.intern(ExprNode::new(domain, ExprKind::Add(non_zero)))
    }

    /// Multiply multiple factor expressions together with zero/unity folding and lossless constant folding.
    pub fn mul<I: IntoIterator<Item = ExprId>>(&self, factors: I) -> ExprId {
        let vec: SmallVec<[ExprId; 4]> = factors.into_iter().collect();
        let mut non_one: SmallVec<[ExprId; 4]> = SmallVec::new();
        let mut const_acc: Option<Number> = None;

        for &f in &vec {
            let node = self.get(f);
            if let ExprKind::Number(n) = &node.kind {
                if n.is_zero() {
                    return self.integer(0);
                }
                if n.is_one() {
                    continue;
                }
                if n.is_lossless() {
                    if let Some(acc) = &const_acc {
                        if let Some(prod) = acc.mul_lossless(n) {
                            const_acc = Some(prod);
                            continue;
                        }
                    } else {
                        const_acc = Some(n.clone());
                        continue;
                    }
                }
            }
            non_one.push(f);
        }

        if let Some(acc) = const_acc {
            if acc.is_zero() {
                return self.integer(0);
            }
            if !acc.is_one() {
                let domain = match &acc {
                    Number::Integer(_) | Number::BigInteger(_) => Domain::Integers,
                    _ => Domain::Rationals,
                };
                let acc_id = self.intern(ExprNode::new(domain, ExprKind::Number(acc)));
                non_one.insert(0, acc_id);
            }
        }

        if non_one.is_empty() {
            return self.integer(1);
        }
        if non_one.len() == 1 {
            return non_one[0];
        }
        let domain = self.infer_poly_domain(&non_one);
        self.intern(ExprNode::new(domain, ExprKind::Mul(non_one)))
    }

    /// Raise base expression to exponent power.
    pub fn pow(&self, base: ExprId, exp: ExprId) -> ExprId {
        let b_node = self.get(base);
        self.intern(ExprNode::new(b_node.domain, ExprKind::Pow(base, exp)))
    }

    /// Divide numerator by denominator.
    pub fn div(&self, num: ExprId, den: ExprId) -> ExprId {
        let n_node = self.get(num);
        let den_node = self.get(den);
        if let ExprKind::Number(n) = &den_node.kind
            && n.is_one()
        {
            return num;
        }
        if let (ExprKind::Number(Number::Integer(n)), ExprKind::Number(Number::Integer(d))) =
            (&n_node.kind, &den_node.kind)
            && *d != 0
        {
            return self.rational(*n, *d);
        }
        self.intern(ExprNode::new(n_node.domain, ExprKind::Div(num, den)))
    }

    /// Subtract rhs from lhs.
    pub fn sub(&self, lhs: ExprId, rhs: ExprId) -> ExprId {
        let neg_rhs = self.neg(rhs);
        self.add([lhs, neg_rhs])
    }

    /// Unary negation of expression.
    pub fn neg(&self, inner: ExprId) -> ExprId {
        let i_node = self.get(inner);
        if let ExprKind::Number(n) = &i_node.kind {
            match n {
                Number::Integer(i) => {
                    if let Some(neg_i) = i.checked_neg() {
                        return self.integer(neg_i);
                    }
                }
                Number::Rational(num, den) => {
                    if let Some(neg_num) = num.checked_neg() {
                        return self.rational(neg_num, *den);
                    }
                }
                _ => {}
            }
        }
        self.intern(ExprNode::new(i_node.domain, ExprKind::Neg(inner)))
    }

    /// Call a symbolic function with arguments.
    pub fn function<I: IntoIterator<Item = ExprId>>(&self, name: &str, args: I) -> ExprId {
        let sym_id = self.symbols.get_or_intern(name);
        let arg_vec: SmallVec<[ExprId; 2]> = args.into_iter().collect();
        let domain = self.infer_poly_domain(&arg_vec);
        self.intern(ExprNode::new(
            domain,
            ExprKind::Function {
                name: sym_id,
                args: arg_vec,
            },
        ))
    }

    /// Derivative operator node.
    pub fn derivative(&self, expr: ExprId, wrt: &str, order: u32) -> ExprId {
        let wrt_sym = self.symbols.get_or_intern(wrt);
        let node = self.get(expr);
        self.intern(ExprNode::new(
            node.domain,
            ExprKind::Derivative {
                expr,
                wrt: wrt_sym,
                order,
            },
        ))
    }

    /// Integral operator node.
    pub fn integral(
        &self,
        expr: ExprId,
        wrt: &str,
        lower: Option<ExprId>,
        upper: Option<ExprId>,
    ) -> ExprId {
        let wrt_sym = self.symbols.get_or_intern(wrt);
        let node = self.get(expr);
        self.intern(ExprNode::new(
            node.domain,
            ExprKind::Integral {
                expr,
                wrt: wrt_sym,
                lower,
                upper,
            },
        ))
    }

    /// Create symbolic repeated summation node: \sum_{var=lower}^{upper} body
    pub fn sum(
        &self,
        body: ExprId,
        var: SymbolId,
        lower: Option<ExprId>,
        upper: Option<ExprId>,
    ) -> ExprId {
        tracing::debug!(body = ?body, var = ?var, "Creating symbolic summation node");
        let node = self.get(body);
        self.intern(ExprNode::new(
            node.domain,
            ExprKind::Sum {
                body,
                var,
                lower,
                upper,
            },
        ))
    }

    /// Create symbolic repeated product node: \prod_{var=lower}^{upper} body
    pub fn product(
        &self,
        body: ExprId,
        var: SymbolId,
        lower: Option<ExprId>,
        upper: Option<ExprId>,
    ) -> ExprId {
        tracing::debug!(body = ?body, var = ?var, "Creating symbolic product node");
        let node = self.get(body);
        self.intern(ExprNode::new(
            node.domain,
            ExprKind::Product {
                body,
                var,
                lower,
                upper,
            },
        ))
    }

    /// Create symbolic tensor contraction node over dummy index pairs.
    pub fn tensor_contraction(
        &self,
        tensor_a: ExprId,
        tensor_b: ExprId,
        contracted_indices: Vec<(SymbolId, SymbolId)>,
    ) -> ExprId {
        tracing::debug!(tensor_a = ?tensor_a, tensor_b = ?tensor_b, "Creating tensor contraction node");
        let node_a = self.get(tensor_a);
        self.intern(ExprNode::new(
            node_a.domain,
            ExprKind::TensorContraction {
                tensor_a,
                tensor_b,
                contracted_indices,
            },
        ))
    }

    /// Matrix node.
    pub fn matrix(&self, rows: usize, cols: usize, elements: Vec<ExprId>) -> ExprId {
        let elem_domain = self.infer_poly_domain(&elements);
        let mat_domain = Domain::Matrix {
            rows,
            cols,
            element_domain: Box::new(elem_domain),
        };
        self.intern(ExprNode::new(
            mat_domain,
            ExprKind::Matrix {
                rows,
                cols,
                elements,
            },
        ))
    }

    /// Relational node (e.g. lhs = rhs, lhs < rhs, etc.).
    pub fn relational(&self, op: RelOp, lhs: ExprId, rhs: ExprId) -> ExprId {
        let dom = self.infer_poly_domain(&[lhs, rhs]);
        self.intern(ExprNode::new(dom, ExprKind::Relational { op, lhs, rhs }))
    }

    /// Substitute symbol `target` with expression `replacement` in `expr`.
    pub fn substitute(&self, expr: ExprId, target: crate::SymbolId, replacement: ExprId) -> ExprId {
        tracing::debug!(expr = ?expr, target = ?target, replacement = ?replacement, "Performing AST node substitution");
        let node = self.get(expr);
        match &node.kind {
            ExprKind::Symbol(s) => {
                if *s == target {
                    replacement
                } else {
                    expr
                }
            }
            ExprKind::Add(terms) => {
                let subbed: SmallVec<[ExprId; 4]> = terms
                    .iter()
                    .map(|&t| self.substitute(t, target, replacement))
                    .collect();
                self.add(subbed)
            }
            ExprKind::Mul(factors) => {
                let subbed: SmallVec<[ExprId; 4]> = factors
                    .iter()
                    .map(|&f| self.substitute(f, target, replacement))
                    .collect();
                self.mul(subbed)
            }
            ExprKind::Sub(l, r) => {
                let sl = self.substitute(*l, target, replacement);
                let sr = self.substitute(*r, target, replacement);
                self.sub(sl, sr)
            }
            ExprKind::Div(num, den) => {
                let sn = self.substitute(*num, target, replacement);
                let sd = self.substitute(*den, target, replacement);
                self.div(sn, sd)
            }
            ExprKind::Pow(b, e) => {
                let sb = self.substitute(*b, target, replacement);
                let se = self.substitute(*e, target, replacement);
                self.pow(sb, se)
            }
            ExprKind::Neg(inner) => {
                let si = self.substitute(*inner, target, replacement);
                self.neg(si)
            }
            ExprKind::Function { name, args } => {
                let subbed_args: Vec<ExprId> = args
                    .iter()
                    .map(|&a| self.substitute(a, target, replacement))
                    .collect();
                let fn_name = self.symbols.resolve(*name).unwrap_or_default();
                self.function(&fn_name, subbed_args)
            }
            _ => expr,
        }
    }

    /// Returns `true` if expression or any sub-node contains variable symbol `target`.
    pub fn has_symbol(&self, expr: ExprId, target: crate::SymbolId) -> bool {
        let node = self.get(expr);
        match &node.kind {
            ExprKind::Symbol(s) => *s == target,
            ExprKind::Add(terms) => terms.iter().any(|&t| self.has_symbol(t, target)),
            ExprKind::Mul(factors) => factors.iter().any(|&f| self.has_symbol(f, target)),
            ExprKind::Sub(l, r) | ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                self.has_symbol(*l, target) || self.has_symbol(*r, target)
            }
            ExprKind::Neg(inner) => self.has_symbol(*inner, target),
            ExprKind::Function { args, .. } => args.iter().any(|&a| self.has_symbol(a, target)),
            ExprKind::Derivative { expr, wrt, .. } => {
                *wrt == target || self.has_symbol(*expr, target)
            }
            ExprKind::Integral {
                expr,
                wrt,
                lower,
                upper,
            } => {
                (*wrt != target && self.has_symbol(*expr, target))
                    || lower.is_some_and(|l| self.has_symbol(l, target))
                    || upper.is_some_and(|u| self.has_symbol(u, target))
                    || (lower.is_none()
                        && upper.is_none()
                        && (*wrt == target || self.has_symbol(*expr, target)))
            }
            ExprKind::Matrix { elements, .. } => {
                elements.iter().any(|&e| self.has_symbol(e, target))
            }
            ExprKind::Relational { lhs, rhs, .. } => {
                self.has_symbol(*lhs, target) || self.has_symbol(*rhs, target)
            }
            ExprKind::SetOperation { args, .. } => {
                args.iter().any(|&a| self.has_symbol(a, target))
            }
            ExprKind::Sum {
                body,
                var,
                lower,
                upper,
            }
            | ExprKind::Product {
                body,
                var,
                lower,
                upper,
            } => {
                (*var != target && self.has_symbol(*body, target))
                    || lower.is_some_and(|l| self.has_symbol(l, target))
                    || upper.is_some_and(|u| self.has_symbol(u, target))
            }
            ExprKind::TensorContraction {
                tensor_a,
                tensor_b,
                ..
            } => self.has_symbol(*tensor_a, target) || self.has_symbol(*tensor_b, target),
            ExprKind::Number(_) => false,
        }
    }

    /// Helper to infer domain resulting from combining expressions.
    fn infer_poly_domain(&self, ids: &[ExprId]) -> Domain {
        if ids.is_empty() {
            return Domain::Reals;
        }
        let mut dom = self.get(ids[0]).domain;
        for id in &ids[1..] {
            let other_dom = self.get(*id).domain;
            if dom.is_subdomain_of(&other_dom) {
                dom = other_dom;
            }
        }
        dom
    }
}
