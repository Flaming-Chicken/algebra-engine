//! E-Graph pattern matching and algebraic simplification engine using `egg`.
//!
//! Provides non-greedy equivalence graph simplification over symbolic expressions,
//! including associative-commutative rings, matrix identities, and hypercomplex (quaternion) identities.
//!
//! ## References
//! - Willsey, M., Nandi, C., Wang, Y. R., Flatt, O., Tatlock, Z., & Panchekha, P. (2021). `egg`: Fast and extensible equality saturation. *Proc. ACM Program. Lang.*, 5(POPL), 1–29.
//! - Nelson, G., & Oppen, D. C. (1980). Fast decision procedures based on congruence closure. *J. ACM*, 27(2), 356–364.

use algebra_core::{ExprGraph, ExprId, ExprKind, Number, StepByStepResult};
use egg::{CostFunction, Id, RecExpr, Rewrite, Runner, define_language};
use thiserror::Error;

/// Errors produced during E-Graph simplification.
#[derive(Debug, Error)]
pub enum SimplifyError {
    /// Generic or custom simplification failure message.
    #[error("Simplification error: {0}")]
    Custom(String),
}

/// Specialized Result alias for simplification operations.
pub type SimplifyResult<T> = Result<T, SimplifyError>;

/// The deepest expression the recursive simplifier accepts. Simplification recurses once per level of nesting, so
/// an unbounded depth is a stack overflow, which aborts the process; 64 levels is far beyond what is written by
/// hand and leaves a wide margin on a 1 MiB stack even in an unoptimised build.
pub const MAX_SIMPLIFY_DEPTH: usize = 64;

/// The nesting depth of an expression (a leaf has depth 1), computed without recursion.
pub fn expression_depth(graph: &ExprGraph, root: ExprId) -> usize {
    use std::collections::HashMap;
    let mut depth: HashMap<ExprId, usize> = HashMap::new();
    let mut stack = vec![(root, false)];
    while let Some((id, children_done)) = stack.pop() {
        if depth.contains_key(&id) {
            continue;
        }
        let children = graph.get(id).children();
        if children_done || children.is_empty() {
            let below = children
                .iter()
                .map(|c| depth.get(c).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            depth.insert(id, below + 1);
        } else {
            stack.push((id, true));
            for c in children {
                if !depth.contains_key(&c) {
                    stack.push((c, false));
                }
            }
        }
    }
    depth.get(&root).copied().unwrap_or(1)
}

pub mod bridges;
pub mod conditions;
pub mod detector;
pub mod dynamic;
pub mod probabilistic;
pub mod transformations;
pub mod trig_expand;

pub use bridges::{
    AskeySchemeGroup, BridgeRouter, EulerBridgeGroup, HyperbolicIdentityGroup, IdentityGroup,
    ItoNilpotentBridgeGroup, LieAlgebraGroup, TransformClusterGroup, TrigIdentityGroup,
};
pub use conditions::{Condition, ConditionKind, ConditionedResult};
pub use detector::{DetectedFeatures, DomainFeatureDetector};
pub use dynamic::{AxiomaticRuleGenerator, DynamicRuleSelector};
pub use probabilistic::{SCHWARTZ_ZIPPEL_PRIME, SchwartzZippel};
pub use transformations::AlgebraicTransformations;

define_language! {
    pub enum SymbolicLang {
        Num(i64),
        "I" = IdentityMatrix,
        "qi" = QuaternionI,
        "qj" = QuaternionJ,
        "qk" = QuaternionK,
        "dW" = SymbolDW,
        "dt" = SymbolDT,
        Symbol(String),
        "+" = Add([Id; 2]),
        "*" = Mul([Id; 2]),
        "-" = Sub([Id; 2]),
        "/" = Div([Id; 2]),
        "^" = Pow([Id; 2]),
        "neg" = Neg(Id),
        "fn" = Fn(Box<[Id]>),
    }
}

/// Custom Cost Function for extracting the simplest AST from an E-Graph.
///
/// Penalizes complex operators like division, exponentiation, and nested functions
/// while favoring basic numbers and symbols.
#[derive(Default)]
pub struct AlgebraicCostFunction;

impl CostFunction<SymbolicLang> for AlgebraicCostFunction {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &SymbolicLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let node_cost = match enode {
            SymbolicLang::Num(_) | SymbolicLang::Symbol(_) => 1,
            SymbolicLang::IdentityMatrix
            | SymbolicLang::QuaternionI
            | SymbolicLang::QuaternionJ
            | SymbolicLang::QuaternionK
            | SymbolicLang::SymbolDW
            | SymbolicLang::SymbolDT => 1,
            SymbolicLang::Neg(_) => 1,
            SymbolicLang::Add(_) | SymbolicLang::Sub(_) => 2,
            SymbolicLang::Mul(_) => 3,
            SymbolicLang::Div(_) => 5,
            SymbolicLang::Pow(_) => 8,
            SymbolicLang::Fn(_) => 10,
        };

        match enode {
            SymbolicLang::Num(_)
            | SymbolicLang::Symbol(_)
            | SymbolicLang::IdentityMatrix
            | SymbolicLang::QuaternionI
            | SymbolicLang::QuaternionJ
            | SymbolicLang::QuaternionK
            | SymbolicLang::SymbolDW
            | SymbolicLang::SymbolDT => node_cost,
            SymbolicLang::Neg(a) => node_cost + costs(*a),
            SymbolicLang::Add([a, b])
            | SymbolicLang::Sub([a, b])
            | SymbolicLang::Mul([a, b])
            | SymbolicLang::Div([a, b])
            | SymbolicLang::Pow([a, b]) => node_cost + costs(*a) + costs(*b),
            SymbolicLang::Fn(args) => node_cost + args.iter().map(|&id| costs(id)).sum::<usize>(),
        }
    }
}

/// Cost function that strongly penalizes AST node count.
#[derive(Default)]
pub struct NodeCountCostFunction;

impl CostFunction<SymbolicLang> for NodeCountCostFunction {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &SymbolicLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            SymbolicLang::Num(_)
            | SymbolicLang::Symbol(_)
            | SymbolicLang::IdentityMatrix
            | SymbolicLang::QuaternionI
            | SymbolicLang::QuaternionJ
            | SymbolicLang::QuaternionK
            | SymbolicLang::SymbolDW
            | SymbolicLang::SymbolDT => 1,
            SymbolicLang::Neg(a) => 1 + costs(*a),
            SymbolicLang::Add([a, b])
            | SymbolicLang::Sub([a, b])
            | SymbolicLang::Mul([a, b])
            | SymbolicLang::Div([a, b])
            | SymbolicLang::Pow([a, b]) => 1 + costs(*a) + costs(*b),
            SymbolicLang::Fn(args) => 1 + args.iter().map(|&id| costs(id)).sum::<usize>(),
        }
    }
}

/// Cost function that minimizes arithmetic operations (+, -, *, /, ^).
#[derive(Default)]
pub struct OpCountCostFunction;

impl CostFunction<SymbolicLang> for OpCountCostFunction {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &SymbolicLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let is_op = matches!(
            enode,
            SymbolicLang::Add(_)
                | SymbolicLang::Sub(_)
                | SymbolicLang::Mul(_)
                | SymbolicLang::Div(_)
                | SymbolicLang::Pow(_)
                | SymbolicLang::Neg(_)
        );
        let op_weight = if is_op { 10 } else { 1 };

        match enode {
            SymbolicLang::Num(_)
            | SymbolicLang::Symbol(_)
            | SymbolicLang::IdentityMatrix
            | SymbolicLang::QuaternionI
            | SymbolicLang::QuaternionJ
            | SymbolicLang::QuaternionK
            | SymbolicLang::SymbolDW
            | SymbolicLang::SymbolDT => 1,
            SymbolicLang::Neg(a) => op_weight + costs(*a),
            SymbolicLang::Add([a, b])
            | SymbolicLang::Sub([a, b])
            | SymbolicLang::Mul([a, b])
            | SymbolicLang::Div([a, b])
            | SymbolicLang::Pow([a, b]) => op_weight + costs(*a) + costs(*b),
            SymbolicLang::Fn(args) => 5 + args.iter().map(|&id| costs(id)).sum::<usize>(),
        }
    }
}

/// Cost function that minimizes leaf variable and numeric nodes.
#[derive(Default)]
pub struct LeafCountCostFunction;

impl CostFunction<SymbolicLang> for LeafCountCostFunction {
    type Cost = usize;

    fn cost<C>(&mut self, enode: &SymbolicLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        match enode {
            SymbolicLang::Num(_)
            | SymbolicLang::Symbol(_)
            | SymbolicLang::IdentityMatrix
            | SymbolicLang::QuaternionI
            | SymbolicLang::QuaternionJ
            | SymbolicLang::QuaternionK
            | SymbolicLang::SymbolDW
            | SymbolicLang::SymbolDT => 1,
            SymbolicLang::Add([a, b])
            | SymbolicLang::Sub([a, b])
            | SymbolicLang::Mul([a, b])
            | SymbolicLang::Div([a, b])
            | SymbolicLang::Pow([a, b]) => costs(*a) + costs(*b),
            SymbolicLang::Neg(a) => costs(*a),
            SymbolicLang::Fn(args) => args.iter().map(|&id| costs(id)).sum::<usize>(),
        }
    }
}

/// Simplification Engine using Equivalence Graphs (E-Graphs).
pub struct Simplifier;

impl Simplifier {
    /// Simplify an expression from `ExprGraph` using dynamic, conflict-free algebraic rewrite rules.
    ///
    /// # Arguments
    /// * `graph` - Reference to the core Arena `ExprGraph`.
    /// * `root` - The root `ExprId` to simplify.
    ///
    /// # Returns
    /// An `ExprId` pointing to the simplified, canonicalized expression in `graph`.
    pub fn simplify(graph: &ExprGraph, root: ExprId) -> SimplifyResult<ExprId> {
        Self::simplify_with_goal(
            graph,
            root,
            algebra_core::domain::GoalDomain::ConserveOriginal,
        )
    }

    /// Simplify an expression with formal probabilistic/deterministic verification certificate.
    pub fn simplify_certified(
        graph: &ExprGraph,
        root: ExprId,
        target_epsilon: f64,
    ) -> SimplifyResult<algebra_core::probabilistic::Solution<ExprId>> {
        let simplified = Self::simplify(graph, root)?;
        let verif = algebra_core::probabilistic::ProbabilisticVerifier::verify_equivalence(
            graph,
            root,
            simplified,
            target_epsilon,
        );
        if verif.is_deterministic() && *verif.as_ref() {
            Ok(algebra_core::probabilistic::Solution::Deterministic(
                simplified,
            ))
        } else {
            Ok(algebra_core::probabilistic::Solution::Probabilistic {
                result: simplified,
                error_probability_upper_bound: verif.error_probability().max(target_epsilon),
                sample_count: verif.sample_count(),
                method: "E-Graph Saturation & Schwartz-Zippel CRT",
            })
        }
    }

    /// Simplify an expression towards an explicit target `GoalDomain` using dynamic bridge routing.
    pub fn simplify_with_goal(
        graph: &ExprGraph,
        root: ExprId,
        goal: algebra_core::domain::GoalDomain,
    ) -> SimplifyResult<ExprId> {
        let empty = crate::logic::AssumptionsContext::new();
        Self::simplify_with_goal_and_assumptions(graph, root, goal, &empty)
    }

    /// Simplify an expression respecting ambient symbol assumptions (e.g. `x > 0`, `n in Integers`).
    pub fn simplify_with_assumptions(
        graph: &ExprGraph,
        root: ExprId,
        assumptions: &crate::logic::AssumptionsContext,
    ) -> SimplifyResult<ExprId> {
        Self::simplify_with_goal_and_assumptions(
            graph,
            root,
            algebra_core::domain::GoalDomain::ConserveOriginal,
            assumptions,
        )
    }

    /// Simplify an expression towards an explicit target `GoalDomain` under symbol assumptions.
    pub fn simplify_with_goal_and_assumptions(
        graph: &ExprGraph,
        root: ExprId,
        goal: algebra_core::domain::GoalDomain,
        assumptions: &crate::logic::AssumptionsContext,
    ) -> SimplifyResult<ExprId> {
        // The depth check comes first: everything below recurses once per level.
        let depth = expression_depth(graph, root);
        if depth > MAX_SIMPLIFY_DEPTH {
            return Err(SimplifyError::Custom(format!(
                "the expression is nested {depth} levels deep; the limit is {MAX_SIMPLIFY_DEPTH}"
            )));
        }
        // `gamma(5)` is 24, `gamma(1/2)` is sqrt(pi), `zeta(2)` is pi^2/6: exact values first.
        let root = crate::special_exact::evaluate(graph, root);
        let node = graph.get(root);
        if let ExprKind::Relational { op, lhs, rhs } = &node.kind {
            let simp_lhs = Self::simplify_with_goal_and_assumptions(graph, *lhs, goal, assumptions)
                .unwrap_or(*lhs);
            let simp_rhs = Self::simplify_with_goal_and_assumptions(graph, *rhs, goal, assumptions)
                .unwrap_or(*rhs);
            return Ok(graph.relational(*op, simp_lhs, simp_rhs));
        }

        let pre = pre_simplify_algebraic_with_assumptions(graph, root, assumptions);
        tracing::debug!(root_id = ?pre, goal = ?goal, "Starting E-Graph simplification");
        let mut rec_expr = RecExpr::default();
        let best_expr_id = match Self::expr_to_egg(graph, pre, &mut rec_expr) {
            Ok(_) => {
                let (rules, _features) = DynamicRuleSelector::select_rules(graph, pre, goal);
                let runner = Self::bounded_runner(&rec_expr).run(&rules);
                let extractor = egg::Extractor::new(&runner.egraph, AlgebraicCostFunction);
                let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);
                tracing::info!(best_cost = ?best_cost, "E-Graph saturation and cost extraction completed");
                Self::egg_to_expr(graph, &best_expr).unwrap_or(pre)
            }
            Err(_) => pre,
        };

        Ok(post_simplify_algebraic_with_assumptions(
            graph,
            best_expr_id,
            assumptions,
        ))
    }

    fn bounded_runner(rec_expr: &RecExpr<SymbolicLang>) -> Runner<SymbolicLang, ()> {
        Runner::default()
            .with_node_limit(15_000)
            .with_iter_limit(30)
            .with_time_limit(std::time::Duration::from_millis(250))
            .with_expr(rec_expr)
    }

    fn minimal_ring_rules() -> Vec<Rewrite<SymbolicLang, ()>> {
        vec![
            egg::rewrite!("add-zero-r"; "(+ ?x 0)" => "?x"),
            egg::rewrite!("add-zero-l"; "(+ 0 ?x)" => "?x"),
            egg::rewrite!("mul-one-r";  "(* ?x 1)" => "?x"),
            egg::rewrite!("mul-one-l";  "(* 1 ?x)" => "?x"),
            egg::rewrite!("mul-zero-r"; "(* ?x 0)" => "0"),
            egg::rewrite!("mul-zero-l"; "(* 0 ?x)" => "0"),
            egg::rewrite!("sub-self";   "(- ?x ?x)" => "0"),
            egg::rewrite!("add-comm";   "(+ ?x ?y)" => "(+ ?y ?x)"),
            egg::rewrite!("mul-comm";   "(* ?x ?y)" => "(* ?y ?x)"),
            egg::rewrite!("add-assoc";  "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
            egg::rewrite!("mul-assoc";  "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        ]
    }

    /// Extract simplified representation minimizing total AST node count (smallest tree depth/size).
    pub fn min_tree(graph: &ExprGraph, root: ExprId) -> SimplifyResult<ExprId> {
        let mut rec_expr = RecExpr::default();
        let _egg_root = Self::expr_to_egg(graph, root, &mut rec_expr)?;
        let rules = Self::minimal_ring_rules();
        let runner = Self::bounded_runner(&rec_expr).run(&rules);
        let extractor = egg::Extractor::new(&runner.egraph, NodeCountCostFunction);
        let (_, best_expr) = extractor.find_best(runner.roots[0]);
        Self::egg_to_expr(graph, &best_expr)
    }

    /// Extract simplified representation minimizing total arithmetic operations (+, -, *, /, ^).
    pub fn min_ops(graph: &ExprGraph, root: ExprId) -> SimplifyResult<ExprId> {
        let mut rec_expr = RecExpr::default();
        let _egg_root = Self::expr_to_egg(graph, root, &mut rec_expr)?;
        let rules = Self::minimal_ring_rules();
        let runner = Self::bounded_runner(&rec_expr).run(&rules);
        let extractor = egg::Extractor::new(&runner.egraph, OpCountCostFunction);
        let (_, best_expr) = extractor.find_best(runner.roots[0]);
        Self::egg_to_expr(graph, &best_expr)
    }

    /// Extract simplified representation minimizing total leaf symbols and constants.
    pub fn min_leaf(graph: &ExprGraph, root: ExprId) -> SimplifyResult<ExprId> {
        let mut rec_expr = RecExpr::default();
        let _egg_root = Self::expr_to_egg(graph, root, &mut rec_expr)?;
        let rules = Self::minimal_ring_rules();
        let runner = Self::bounded_runner(&rec_expr).run(&rules);
        let extractor = egg::Extractor::new(&runner.egraph, LeafCountCostFunction);
        let (_, best_expr) = extractor.find_best(runner.roots[0]);
        Self::egg_to_expr(graph, &best_expr)
    }

    /// Simplify an expression from `ExprGraph` using algebraic rewrite rules,
    /// returning the result alongside explicit step-by-step educational derivation steps.
    pub fn simplify_with_steps(
        graph: &ExprGraph,
        root: ExprId,
    ) -> SimplifyResult<StepByStepResult<ExprId>> {
        let simplified_root = Self::simplify(graph, root)?;
        let mut steps = Vec::new();

        if simplified_root != root {
            steps.push(algebra_core::MathStep::new(
                1,
                "E-Graph Pattern Saturation",
                "Applied non-greedy associative-commutative ring and algebra rewrite rules",
                root,
                simplified_root,
            ));
        }

        Ok(StepByStepResult::new(simplified_root, steps))
    }

    fn expr_to_egg(
        graph: &ExprGraph,
        id: ExprId,
        rec: &mut RecExpr<SymbolicLang>,
    ) -> SimplifyResult<Id> {
        let node = graph.get(id);
        match &node.kind {
            ExprKind::Number(Number::Integer(i)) => Ok(rec.add(SymbolicLang::Num(*i))),
            ExprKind::Number(Number::BigInteger(b)) => {
                use num_traits::ToPrimitive;
                if let Some(i) = b.to_i64() {
                    Ok(rec.add(SymbolicLang::Num(i)))
                } else {
                    Ok(rec.add(SymbolicLang::Symbol(b.to_string())))
                }
            }
            ExprKind::Number(Number::Rational(num, den)) => {
                if *den == 1 {
                    Ok(rec.add(SymbolicLang::Num(*num)))
                } else {
                    let n = rec.add(SymbolicLang::Num(*num));
                    let d = rec.add(SymbolicLang::Num(*den));
                    Ok(rec.add(SymbolicLang::Div([n, d])))
                }
            }
            ExprKind::Number(Number::BigRational(r)) => {
                use num_traits::ToPrimitive;
                if let (Some(n), Some(d)) = (r.numer().to_i64(), r.denom().to_i64()) {
                    if d == 1 {
                        Ok(rec.add(SymbolicLang::Num(n)))
                    } else {
                        let n_id = rec.add(SymbolicLang::Num(n));
                        let d_id = rec.add(SymbolicLang::Num(d));
                        Ok(rec.add(SymbolicLang::Div([n_id, d_id])))
                    }
                } else {
                    Ok(rec.add(SymbolicLang::Symbol(r.to_string())))
                }
            }
            ExprKind::Number(Number::Scientific { mantissa, exponent }) => {
                let n = Number::scientific(*mantissa, *exponent);
                if let Number::Rational(num, den) = n {
                    let n_id = rec.add(SymbolicLang::Num(num));
                    let d_id = rec.add(SymbolicLang::Num(den));
                    Ok(rec.add(SymbolicLang::Div([n_id, d_id])))
                } else if let Number::Integer(i) = n {
                    Ok(rec.add(SymbolicLang::Num(i)))
                } else {
                    let m_id = rec.add(SymbolicLang::Num(*mantissa));
                    let ten_id = rec.add(SymbolicLang::Num(10));
                    let exp_id = rec.add(SymbolicLang::Num(*exponent as i64));
                    let pow_id = rec.add(SymbolicLang::Pow([ten_id, exp_id]));
                    Ok(rec.add(SymbolicLang::Mul([m_id, pow_id])))
                }
            }
            ExprKind::Number(Number::Float(bits)) => {
                let f = f64::from_bits(*bits);
                let decomp = Number::from_f64_lossless(f);
                if let Number::Integer(i) = decomp {
                    Ok(rec.add(SymbolicLang::Num(i)))
                } else if let Number::Rational(num, den) = decomp {
                    let n = rec.add(SymbolicLang::Num(num));
                    let d = rec.add(SymbolicLang::Num(den));
                    Ok(rec.add(SymbolicLang::Div([n, d])))
                } else if f.fract() == 0.0 {
                    Ok(rec.add(SymbolicLang::Num(f as i64)))
                } else {
                    Ok(rec.add(SymbolicLang::Num(f.round() as i64)))
                }
            }
            ExprKind::Number(Number::Constant(c)) => match c {
                algebra_core::Constant::I => Ok(rec.add(SymbolicLang::QuaternionI)),
                algebra_core::Constant::Pi => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_pi".into())))
                }
                algebra_core::Constant::E => Ok(rec.add(SymbolicLang::Symbol("__const_e".into()))),
                algebra_core::Constant::EulerGamma => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_gamma".into())))
                }
                algebra_core::Constant::GoldenRatio => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_phi".into())))
                }
                algebra_core::Constant::Apery => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_apery".into())))
                }
                algebra_core::Constant::Catalan => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_catalan".into())))
                }
                algebra_core::Constant::Infinity => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_inf".into())))
                }
                algebra_core::Constant::NegInfinity => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_neginf".into())))
                }
                algebra_core::Constant::Undefined => {
                    Ok(rec.add(SymbolicLang::Symbol("__const_undef".into())))
                }
            },
            ExprKind::Symbol(sym) => {
                let name = graph
                    .symbols
                    .resolve(*sym)
                    .unwrap_or_else(|| "x".to_string());
                match name.as_str() {
                    "I" => Ok(rec.add(SymbolicLang::IdentityMatrix)),
                    "i" | "qi" => Ok(rec.add(SymbolicLang::QuaternionI)),
                    "qj" => Ok(rec.add(SymbolicLang::QuaternionJ)),
                    "qk" => Ok(rec.add(SymbolicLang::QuaternionK)),
                    "dW" | "dW_t" => Ok(rec.add(SymbolicLang::SymbolDW)),
                    "dt" => Ok(rec.add(SymbolicLang::SymbolDT)),
                    _ => Ok(rec.add(SymbolicLang::Symbol(name))),
                }
            }
            ExprKind::Add(terms) => {
                if terms.is_empty() {
                    return Ok(rec.add(SymbolicLang::Num(0)));
                }
                let mut current = Self::expr_to_egg(graph, terms[0], rec)?;
                for &t in &terms[1..] {
                    let next = Self::expr_to_egg(graph, t, rec)?;
                    current = rec.add(SymbolicLang::Add([current, next]));
                }
                Ok(current)
            }
            ExprKind::Mul(factors) => {
                if factors.is_empty() {
                    return Ok(rec.add(SymbolicLang::Num(1)));
                }
                let mut current = Self::expr_to_egg(graph, factors[0], rec)?;
                for &f in &factors[1..] {
                    let next = Self::expr_to_egg(graph, f, rec)?;
                    current = rec.add(SymbolicLang::Mul([current, next]));
                }
                Ok(current)
            }
            ExprKind::Pow(b, e) => {
                let b_id = Self::expr_to_egg(graph, *b, rec)?;
                let e_id = Self::expr_to_egg(graph, *e, rec)?;
                Ok(rec.add(SymbolicLang::Pow([b_id, e_id])))
            }
            ExprKind::Sub(l, r) => {
                let l_id = Self::expr_to_egg(graph, *l, rec)?;
                let r_id = Self::expr_to_egg(graph, *r, rec)?;
                Ok(rec.add(SymbolicLang::Sub([l_id, r_id])))
            }
            ExprKind::Div(num, den) => {
                let n_id = Self::expr_to_egg(graph, *num, rec)?;
                let d_id = Self::expr_to_egg(graph, *den, rec)?;
                Ok(rec.add(SymbolicLang::Div([n_id, d_id])))
            }
            ExprKind::Function { name, args } => {
                let fn_name = graph
                    .symbols
                    .resolve(*name)
                    .unwrap_or_else(|| "f".to_string());
                let mut children = Vec::with_capacity(args.len() + 1);
                children.push(rec.add(SymbolicLang::Symbol(fn_name)));
                for &a in args {
                    children.push(Self::expr_to_egg(graph, a, rec)?);
                }
                Ok(rec.add(SymbolicLang::Fn(children.into_boxed_slice())))
            }
            ExprKind::Neg(inner) => {
                let i_id = Self::expr_to_egg(graph, *inner, rec)?;
                Ok(rec.add(SymbolicLang::Neg(i_id)))
            }
            _ => Err(SimplifyError::Custom(format!(
                "Unsupported node for E-graph conversion: {:?}",
                node.kind
            ))),
        }
    }

    fn egg_to_expr(graph: &ExprGraph, rec: &RecExpr<SymbolicLang>) -> SimplifyResult<ExprId> {
        let nodes = rec.as_ref();
        if nodes.is_empty() {
            return Err(SimplifyError::Custom("Empty RecExpr".into()));
        }
        Self::egg_node_to_expr(graph, rec, Id::from(nodes.len() - 1))
    }

    fn egg_node_to_expr(
        graph: &ExprGraph,
        rec: &RecExpr<SymbolicLang>,
        id: Id,
    ) -> SimplifyResult<ExprId> {
        let node = &rec[id];
        match node {
            SymbolicLang::Num(i) => Ok(graph.integer(*i)),
            SymbolicLang::Symbol(s) => match s.as_str() {
                "__const_pi" => Ok(graph.constant(algebra_core::Constant::Pi)),
                "__const_e" => Ok(graph.constant(algebra_core::Constant::E)),
                "__const_gamma" => Ok(graph.constant(algebra_core::Constant::EulerGamma)),
                "__const_phi" => Ok(graph.constant(algebra_core::Constant::GoldenRatio)),
                "__const_apery" => Ok(graph.constant(algebra_core::Constant::Apery)),
                "__const_catalan" => Ok(graph.constant(algebra_core::Constant::Catalan)),
                "__const_inf" => Ok(graph.constant(algebra_core::Constant::Infinity)),
                "__const_neginf" => Ok(graph.constant(algebra_core::Constant::NegInfinity)),
                "__const_undef" => Ok(graph.constant(algebra_core::Constant::Undefined)),
                _ => Ok(graph.symbol(s)),
            },
            SymbolicLang::IdentityMatrix => Ok(graph.symbol("I")),
            SymbolicLang::QuaternionI => Ok(graph.symbol("i")),
            SymbolicLang::QuaternionJ => Ok(graph.symbol("qj")),
            SymbolicLang::QuaternionK => Ok(graph.symbol("qk")),
            SymbolicLang::SymbolDW => Ok(graph.symbol("dW")),
            SymbolicLang::SymbolDT => Ok(graph.symbol("dt")),
            SymbolicLang::Add([a, b]) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                let e2 = Self::egg_node_to_expr(graph, rec, *b)?;
                Ok(graph.add([e1, e2]))
            }
            SymbolicLang::Mul([a, b]) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                let e2 = Self::egg_node_to_expr(graph, rec, *b)?;
                Ok(graph.mul([e1, e2]))
            }
            SymbolicLang::Sub([a, b]) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                let e2 = Self::egg_node_to_expr(graph, rec, *b)?;
                Ok(graph.sub(e1, e2))
            }
            SymbolicLang::Pow([a, b]) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                let e2 = Self::egg_node_to_expr(graph, rec, *b)?;
                Ok(graph.pow(e1, e2))
            }
            SymbolicLang::Div([a, b]) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                let e2 = Self::egg_node_to_expr(graph, rec, *b)?;
                Ok(graph.div(e1, e2))
            }
            SymbolicLang::Neg(a) => {
                let e1 = Self::egg_node_to_expr(graph, rec, *a)?;
                Ok(graph.neg(e1))
            }
            SymbolicLang::Fn(children) => {
                let Some((&fn_id, arg_ids)) = children.split_first() else {
                    return Err(SimplifyError::Custom("Empty Fn node".into()));
                };
                let fn_name = match &rec[fn_id] {
                    SymbolicLang::Symbol(s) => s.as_str(),
                    _ => return Err(SimplifyError::Custom("Non-symbol Fn head".into())),
                };
                let mut args = Vec::with_capacity(arg_ids.len());
                for &aid in arg_ids {
                    args.push(Self::egg_node_to_expr(graph, rec, aid)?);
                }
                Ok(graph.function(fn_name, args))
            }
        }
    }
}

#[allow(dead_code)]
fn pre_simplify_algebraic(graph: &ExprGraph, id: ExprId) -> ExprId {
    let empty = crate::logic::AssumptionsContext::new();
    pre_simplify_algebraic_with_assumptions(graph, id, &empty)
}

fn pre_simplify_algebraic_with_assumptions(
    graph: &ExprGraph,
    id: ExprId,
    assumptions: &crate::logic::AssumptionsContext,
) -> ExprId {
    use crate::logic::Predicate;
    use num::Integer as _;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, ToPrimitive as _, Zero as _};

    fn is_zero_id(g: &ExprGraph, e: ExprId) -> bool {
        matches!(
            g.get(e).kind,
            ExprKind::Number(Number::Integer(0)) | ExprKind::Number(Number::Rational(0, _))
        )
    }

    fn is_one_id(g: &ExprGraph, e: ExprId) -> bool {
        matches!(
            g.get(e).kind,
            ExprKind::Number(Number::Integer(1)) | ExprKind::Number(Number::Rational(1, 1))
        )
    }

    fn contains_imaginary_unit(g: &ExprGraph, id: ExprId) -> bool {
        match &g.get(id).kind {
            ExprKind::Number(Number::Constant(algebra_core::Constant::I)) => true,
            ExprKind::Symbol(s) => matches!(g.symbols.resolve(*s).as_deref(), Some("i" | "I")),
            ExprKind::Neg(a) => contains_imaginary_unit(g, *a),
            ExprKind::Add(xs) | ExprKind::Mul(xs) => {
                xs.iter().any(|&x| contains_imaginary_unit(g, x))
            }
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                contains_imaginary_unit(g, *a) || contains_imaginary_unit(g, *b)
            }
            ExprKind::Function { args, .. } => args.iter().any(|&a| contains_imaginary_unit(g, a)),
            _ => false,
        }
    }

    fn as_rat(g: &ExprGraph, e: ExprId) -> Option<BigRational> {
        match &g.get(e).kind {
            ExprKind::Number(n) => crate::poly::rational_from_number(n).ok(),
            ExprKind::Neg(inner) => Some(-as_rat(g, *inner)?),
            ExprKind::Div(a, b) => {
                let qa = as_rat(g, *a)?;
                let qb = as_rat(g, *b)?;
                if qb.is_zero() { None } else { Some(qa / qb) }
            }
            _ => None,
        }
    }

    fn simplify_sqrt_int(g: &ExprGraph, radicand: ExprId) -> Option<ExprId> {
        let q = as_rat(g, radicand)?;
        if !q.is_integer() || q.is_negative() {
            return None;
        }
        let n = q.numer().to_u64()?;
        if n == 0 {
            return Some(g.integer(0));
        }
        if n == 1 {
            return Some(g.integer(1));
        }
        let mut sq = 1u64;
        let mut rem = n;
        let mut d = 2u64;
        while d * d <= rem {
            while rem % (d * d) == 0 {
                sq *= d;
                rem /= d * d;
            }
            d += if d == 2 { 1 } else { 2 };
        }
        if rem == 1 {
            return Some(g.integer(sq as i64));
        }
        if sq > 1 {
            let sqrt_rem = g.function("sqrt", [g.integer(rem as i64)]);
            return Some(g.mul([g.integer(sq as i64), sqrt_rem]));
        }
        None
    }

    fn is_imag_unit(g: &ExprGraph, id: ExprId) -> bool {
        matches!(
            g.get(id).kind,
            ExprKind::Number(Number::Constant(algebra_core::Constant::I))
        ) || matches!(
            g.get(id).kind,
            ExprKind::Symbol(s) if matches!(g.symbols.resolve(s).as_deref(), Some("i" | "I"))
        )
    }

    fn extract_pi_rational(g: &ExprGraph, id: ExprId) -> Option<BigRational> {
        match &g.get(id).kind {
            ExprKind::Number(Number::Constant(algebra_core::Constant::Pi)) => {
                Some(BigRational::one())
            }
            ExprKind::Symbol(s) if matches!(g.symbols.resolve(*s).as_deref(), Some("pi" | "π")) => {
                Some(BigRational::one())
            }
            ExprKind::Neg(a) => Some(-extract_pi_rational(g, *a)?),
            ExprKind::Mul(fs) => {
                let mut rat = BigRational::one();
                let mut seen_pi = false;
                for &f in fs {
                    if let Some(q) = as_rat(g, f) {
                        rat *= q;
                    } else {
                        let qp = extract_pi_rational(g, f)?;
                        if seen_pi {
                            return None;
                        }
                        rat *= qp;
                        seen_pi = true;
                    }
                }
                if seen_pi { Some(rat) } else { None }
            }
            ExprKind::Div(a, b) => {
                let qa = extract_pi_rational(g, *a)?;
                let qb = as_rat(g, *b)?;
                if qb.is_zero() { None } else { Some(qa / qb) }
            }
            _ => None,
        }
    }

    fn extract_i_pi_rational(g: &ExprGraph, id: ExprId) -> Option<BigRational> {
        match &g.get(id).kind {
            ExprKind::Mul(fs) => {
                let mut rat = BigRational::one();
                let mut seen_i = false;
                let mut seen_pi = false;
                for &f in fs {
                    if is_imag_unit(g, f) && !seen_i {
                        seen_i = true;
                    } else if let Some(q) = as_rat(g, f) {
                        rat *= q;
                    } else if let Some(qp) = extract_pi_rational(g, f)
                        && !seen_pi
                    {
                        rat *= qp;
                        seen_pi = true;
                    } else {
                        return None;
                    }
                }
                if seen_i && seen_pi { Some(rat) } else { None }
            }
            ExprKind::Neg(a) => Some(-extract_i_pi_rational(g, *a)?),
            ExprKind::Div(a, b) => {
                let qa = extract_i_pi_rational(g, *a)?;
                let qb = as_rat(g, *b)?;
                if qb.is_zero() { None } else { Some(qa / qb) }
            }
            _ => None,
        }
    }

    fn eval_exact_trig_pi(g: &ExprGraph, fn_name: &str, k_pi: &BigRational) -> Option<ExprId> {
        // Reduce k mod 2 into [0, 2)
        let two = BigRational::from_integer(2.into());
        let mut r = k_pi.clone();
        if r >= two || r < BigRational::zero() {
            let floored = (r.clone() / &two).floor();
            r -= floored * &two;
        }
        // Check denominator divides 12 (24 ticks around the unit circle, step = pi/12 -> check step pi/6 & pi/4)
        let scale = BigRational::from_integer(12.into());
        let scaled = &r * &scale;
        if !scaled.is_integer() {
            return None;
        }
        let n24 = scaled.numer().to_i64()?.rem_euclid(24);
        // sin(n * pi / 12) for n in {0,2,3,4,6,8,9,10,12,14,15,16,18,20,21,22}
        let sin_val = |tick: i64| -> Option<ExprId> {
            let t = tick.rem_euclid(24);
            let neg = t > 12;
            let q = if neg { t - 12 } else { t }; // in 0..=12
            let sym_q = if q > 6 { 12 - q } else { q }; // in 0..=6
            let mag = match sym_q {
                0 => return Some(g.integer(0)),
                2 => g.rational(1, 2),
                3 => g.div(g.function("sqrt", [g.integer(2)]), g.integer(2)),
                4 => g.div(g.function("sqrt", [g.integer(3)]), g.integer(2)),
                6 => g.integer(1),
                _ => return None,
            };
            Some(if neg { g.neg(mag) } else { mag })
        };
        match fn_name {
            "sin" => sin_val(n24),
            "cos" => sin_val(n24 + 6),
            "tan" => {
                if n24 % 12 == 6 {
                    return Some(g.constant(algebra_core::Constant::Undefined));
                }
                let q = n24.rem_euclid(12);
                let neg = q > 6;
                let a = if neg { 12 - q } else { q };
                let mag = match a {
                    0 => return Some(g.integer(0)),
                    2 => g.div(g.function("sqrt", [g.integer(3)]), g.integer(3)),
                    3 => g.integer(1),
                    4 => g.function("sqrt", [g.integer(3)]),
                    _ => return None,
                };
                Some(if neg { g.neg(mag) } else { mag })
            }
            _ => None,
        }
    }

    fn split_rat_core(g: &ExprGraph, e: ExprId) -> (BigRational, Vec<ExprId>) {
        if let Some(q) = as_rat(g, e) {
            return (q, Vec::new());
        }
        match &g.get(e).kind {
            ExprKind::Neg(inner) => {
                let (q, core) = split_rat_core(g, *inner);
                (-q, core)
            }
            ExprKind::Mul(fs) => {
                let mut q = BigRational::one();
                let mut core = Vec::new();
                for &f in fs {
                    if let Some(qf) = as_rat(g, f) {
                        q *= qf;
                    } else if let ExprKind::Neg(inn) = g.get(f).kind {
                        q = -q;
                        let (qi, ci) = split_rat_core(g, inn);
                        q *= qi;
                        core.extend(ci);
                    } else {
                        core.push(f);
                    }
                }
                core.sort_by_key(|id| id.0);
                (q, core)
            }
            _ => (BigRational::one(), vec![e]),
        }
    }

    fn fn_sq_arg(g: &ExprGraph, id: ExprId, target_fn: &str) -> Option<ExprId> {
        if let ExprKind::Pow(base, exp) = g.get(id).kind
            && as_rat(g, exp) == Some(BigRational::from_integer(2.into()))
            && let ExprKind::Function { name, args } = g.get(base).kind
            && args.len() == 1
            && g.symbols.resolve(name).as_deref() == Some(target_fn)
        {
            return Some(args[0]);
        }
        None
    }

    fn combine_additive(g: &ExprGraph, signed_items: &[(bool, ExprId)]) -> ExprId {
        let mut const_sum = BigRational::zero();
        let mut map: std::collections::BTreeMap<Vec<ExprId>, BigRational> =
            std::collections::BTreeMap::new();
        for &(is_neg, t) in signed_items {
            if is_zero_id(g, t) {
                continue;
            }
            let (mut q, core) = split_rat_core(g, t);
            if is_neg {
                q = -q;
            }
            if q.is_zero() {
                continue;
            }
            if core.is_empty() {
                const_sum += q;
            } else {
                let entry = map.entry(core.clone()).or_insert_with(BigRational::zero);
                *entry += q;
                if entry.is_zero() {
                    map.remove(&core);
                }
            }
        }

        // Collapse Pythagorean pairs in combined cores: c*sin(u)^2 + c*cos(u)^2 -> c (or * rest_core)
        loop {
            let keys: Vec<Vec<ExprId>> = map.keys().cloned().collect();
            let mut merged = false;
            'outer: for key in keys {
                let Some(c1) = map.get(&key).cloned() else {
                    continue;
                };
                for (pos, &elem) in key.iter().enumerate() {
                    if let Some(u) = fn_sq_arg(g, elem, "sin") {
                        let cos_u_sq = g.pow(g.function("cos", [u]), g.integer(2));
                        let mut partner = key.clone();
                        partner[pos] = cos_u_sq;
                        partner.sort_by_key(|id| id.0);
                        if let Some(c2) = map.get(&partner).cloned()
                            && c1 == c2
                        {
                            map.remove(&key);
                            map.remove(&partner);
                            let mut rest = key.clone();
                            rest.remove(pos);
                            if rest.is_empty() {
                                const_sum += c1;
                            } else {
                                let e = map.entry(rest.clone()).or_insert_with(BigRational::zero);
                                *e += c1;
                                if e.is_zero() {
                                    map.remove(&rest);
                                }
                            }
                            merged = true;
                            break 'outer;
                        }
                    } else if let Some(u) = fn_sq_arg(g, elem, "cosh") {
                        let sinh_u_sq = g.pow(g.function("sinh", [u]), g.integer(2));
                        let mut partner = key.clone();
                        partner[pos] = sinh_u_sq;
                        partner.sort_by_key(|id| id.0);
                        if let Some(c2) = map.get(&partner).cloned()
                            && c1 == -c2
                        {
                            map.remove(&key);
                            map.remove(&partner);
                            let mut rest = key.clone();
                            rest.remove(pos);
                            if rest.is_empty() {
                                const_sum += c1;
                            } else {
                                let e = map.entry(rest.clone()).or_insert_with(BigRational::zero);
                                *e += c1;
                                if e.is_zero() {
                                    map.remove(&rest);
                                }
                            }
                            merged = true;
                            break 'outer;
                        }
                    }
                }
            }
            if !merged {
                break;
            }
        }

        // Handle 1 - cos(u)^2 -> sin(u)^2 and 1 - sin(u)^2 -> cos(u)^2
        if !const_sum.is_zero() {
            let keys: Vec<Vec<ExprId>> = map.keys().cloned().collect();
            for key in keys {
                if key.len() == 1
                    && let Some(c) = map.get(&key).cloned()
                    && c == -const_sum.clone()
                {
                    if let Some(u) = fn_sq_arg(g, key[0], "cos") {
                        let sin_sq = vec![g.pow(g.function("sin", [u]), g.integer(2))];
                        map.remove(&key);
                        const_sum = BigRational::zero();
                        let e = map.entry(sin_sq).or_insert_with(BigRational::zero);
                        *e -= c;
                        break;
                    } else if let Some(u) = fn_sq_arg(g, key[0], "sin") {
                        let cos_sq = vec![g.pow(g.function("cos", [u]), g.integer(2))];
                        map.remove(&key);
                        const_sum = BigRational::zero();
                        let e = map.entry(cos_sq).or_insert_with(BigRational::zero);
                        *e -= c;
                        break;
                    }
                }
            }
        }

        let mut terms = Vec::new();
        for (core, c) in map {
            if c.is_zero() {
                continue;
            }
            let core_expr = if core.len() == 1 {
                core[0]
            } else {
                g.mul(core)
            };
            let term = if c.is_one() {
                core_expr
            } else if c == -BigRational::one() {
                g.neg(core_expr)
            } else {
                g.mul([crate::poly::expr_from_rational(g, &c), core_expr])
            };
            terms.push(term);
        }
        if !const_sum.is_zero() {
            terms.push(crate::poly::expr_from_rational(g, &const_sum));
        }
        if terms.is_empty() {
            g.integer(0)
        } else if terms.len() == 1 {
            terms[0]
        } else {
            g.add(terms)
        }
    }

    fn rec(g: &ExprGraph, e: ExprId, assumptions: &crate::logic::AssumptionsContext) -> ExprId {
        match &g.get(e).kind {
            ExprKind::Neg(inner) => {
                let s = rec(g, *inner, assumptions);
                if is_zero_id(g, s) {
                    return g.integer(0);
                }
                if let ExprKind::Neg(a) = g.get(s).kind {
                    return a;
                }
                if let Some(q) = as_rat(g, s) {
                    return crate::poly::expr_from_rational(g, &(-q));
                }
                g.neg(s)
            }
            ExprKind::Add(terms) => {
                let mut flat = Vec::new();
                fn collect_add_flat(
                    g: &ExprGraph,
                    id: ExprId,
                    neg: bool,
                    assumptions: &crate::logic::AssumptionsContext,
                    out: &mut Vec<(bool, ExprId)>,
                ) {
                    match &g.get(id).kind {
                        ExprKind::Add(sub_ts) => {
                            for &t in sub_ts {
                                collect_add_flat(g, t, neg, assumptions, out);
                            }
                        }
                        ExprKind::Sub(l, r) => {
                            collect_add_flat(g, *l, neg, assumptions, out);
                            collect_add_flat(g, *r, !neg, assumptions, out);
                        }
                        ExprKind::Neg(inner) => {
                            collect_add_flat(g, *inner, !neg, assumptions, out);
                        }
                        _ => out.push((neg, rec(g, id, assumptions))),
                    }
                }
                for &t in terms {
                    let st = rec(g, t, assumptions);
                    collect_add_flat(g, st, false, assumptions, &mut flat);
                }
                combine_additive(g, &flat)
            }
            ExprKind::Sub(a, b) => {
                let sa = rec(g, *a, assumptions);
                let sb = rec(g, *b, assumptions);
                if sa == sb {
                    return g.integer(0);
                }
                if is_zero_id(g, sb) {
                    return sa;
                }
                if is_zero_id(g, sa) {
                    return rec(g, g.neg(sb), assumptions);
                }
                if let (Some(qa), Some(qb)) = (as_rat(g, sa), as_rat(g, sb)) {
                    return crate::poly::expr_from_rational(g, &(qa - qb));
                }
                // sec(u)^2 - tan(u)^2 -> 1, csc(u)^2 - cot(u)^2 -> 1
                if let (Some(u1), Some(u2)) = (fn_sq_arg(g, sa, "sec"), fn_sq_arg(g, sb, "tan"))
                    && u1 == u2
                {
                    return g.integer(1);
                }
                if let (Some(u1), Some(u2)) = (fn_sq_arg(g, sa, "csc"), fn_sq_arg(g, sb, "cot"))
                    && u1 == u2
                {
                    return g.integer(1);
                }
                combine_additive(g, &[(false, sa), (true, sb)])
            }
            ExprKind::Mul(factors) => {
                let mut flat = Vec::new();
                for &f in factors {
                    let sf = rec(g, f, assumptions);
                    match &g.get(sf).kind {
                        ExprKind::Mul(sub_fs) => flat.extend(sub_fs.iter().copied()),
                        _ => flat.push(sf),
                    }
                }
                let mut coeff = BigRational::one();
                let mut rest = Vec::new();
                for f in flat {
                    if is_zero_id(g, f) {
                        return g.integer(0);
                    }
                    if let Some(q) = as_rat(g, f) {
                        coeff *= q;
                    } else if let ExprKind::Neg(inn) = g.get(f).kind {
                        coeff = -coeff;
                        rest.push(inn);
                    } else {
                        rest.push(f);
                    }
                }
                // sqrt(a) * sqrt(b) -> sqrt(a * b) for positive rationals a, b, so the
                // radical simplifier can pull out squares: sqrt(2) * sqrt(8) -> sqrt(16) -> 4.
                let sqrt_of_positive_rational = |g: &ExprGraph, f: ExprId| -> Option<BigRational> {
                    if let ExprKind::Function { name, args } = &g.get(f).kind
                        && args.len() == 1
                        && g.symbols.resolve(*name).as_deref() == Some("sqrt")
                    {
                        return as_rat(g, args[0]).filter(|q| q > &BigRational::zero());
                    }
                    None
                };
                let radicands: Vec<BigRational> = rest
                    .iter()
                    .filter_map(|&f| sqrt_of_positive_rational(g, f))
                    .collect();
                if radicands.len() >= 2 {
                    let product = radicands
                        .into_iter()
                        .fold(BigRational::one(), |acc, q| acc * q);
                    let mut rebuilt: Vec<ExprId> = rest
                        .iter()
                        .copied()
                        .filter(|&f| sqrt_of_positive_rational(g, f).is_none())
                        .collect();
                    if !coeff.is_one() {
                        rebuilt.insert(0, crate::poly::expr_from_rational(g, &coeff));
                    }
                    rebuilt
                        .push(g.function("sqrt", [crate::poly::expr_from_rational(g, &product)]));
                    let combined = if rebuilt.len() == 1 {
                        rebuilt[0]
                    } else {
                        g.mul(rebuilt)
                    };
                    return rec(g, combined, assumptions);
                }
                // exp(a) * exp(b) -> exp(a + b), so that exp(x) * exp(-x) is 1.
                let exp_argument = |g: &ExprGraph, f: ExprId| -> Option<ExprId> {
                    if let ExprKind::Function { name, args } = &g.get(f).kind
                        && args.len() == 1
                        && g.symbols.resolve(*name).as_deref() == Some("exp")
                    {
                        return Some(args[0]);
                    }
                    None
                };
                let exp_args: Vec<ExprId> =
                    rest.iter().filter_map(|&f| exp_argument(g, f)).collect();
                if exp_args.len() >= 2 {
                    let mut rebuilt: Vec<ExprId> = rest
                        .iter()
                        .copied()
                        .filter(|&f| exp_argument(g, f).is_none())
                        .collect();
                    if !coeff.is_one() {
                        rebuilt.insert(0, crate::poly::expr_from_rational(g, &coeff));
                    }
                    rebuilt.push(g.function("exp", [g.add(exp_args)]));
                    let combined = if rebuilt.len() == 1 {
                        rebuilt[0]
                    } else {
                        g.mul(rebuilt)
                    };
                    return rec(g, combined, assumptions);
                }
                // Repeated bases with numeric exponents: x * x -> x^2, x^2 * x^3 -> x^5,
                // x^(1/2) * x^(1/2) -> x. Only rewritten when a base actually repeats, so
                // products without repeats keep their order.
                let base_and_exponent =
                    |g: &ExprGraph, f: ExprId| -> Option<(ExprId, BigRational)> {
                        if as_rat(g, f).is_some() {
                            return None;
                        }
                        if let ExprKind::Pow(b, e) = g.get(f).kind {
                            return as_rat(g, e)
                                .filter(|_| as_rat(g, b).is_none())
                                .map(|q| (b, q));
                        }
                        Some((f, BigRational::one()))
                    };
                let mut grouped: Vec<(ExprId, BigRational)> = Vec::new();
                let mut others: Vec<ExprId> = Vec::new();
                let mut repeated = false;
                for &f in &rest {
                    match base_and_exponent(g, f) {
                        Some((base, exp)) => {
                            if let Some(slot) = grouped.iter_mut().find(|(b, _)| *b == base) {
                                slot.1 += exp;
                                repeated = true;
                            } else {
                                grouped.push((base, exp));
                            }
                        }
                        None => others.push(f),
                    }
                }
                if repeated {
                    rest = grouped
                        .into_iter()
                        .filter(|(_, exp)| !exp.is_zero())
                        .map(|(base, exp)| {
                            if exp.is_one() {
                                base
                            } else {
                                g.pow(base, crate::poly::expr_from_rational(g, &exp))
                            }
                        })
                        .chain(others)
                        .collect();
                }
                // Pairwise trig/reciprocal cancellations inside Mul
                let mut changed = true;
                while changed {
                    changed = false;
                    'pair: for i in 0..rest.len() {
                        for j in 0..rest.len() {
                            if i == j {
                                continue;
                            }
                            if let (
                                ExprKind::Function { name: n1, args: a1 },
                                ExprKind::Function { name: n2, args: a2 },
                            ) = (&g.get(rest[i]).kind, &g.get(rest[j]).kind)
                                && a1.len() == 1
                                && a2.len() == 1
                                && a1[0] == a2[0]
                            {
                                let f1 = g.symbols.resolve(*n1).unwrap_or_default();
                                let f2 = g.symbols.resolve(*n2).unwrap_or_default();
                                let rep = match (f1.as_str(), f2.as_str()) {
                                    ("tan", "cos") => Some(Some(g.function("sin", [a1[0]]))),
                                    ("cot", "sin") => Some(Some(g.function("cos", [a1[0]]))),
                                    ("tanh", "cosh") => Some(Some(g.function("sinh", [a1[0]]))),
                                    ("coth", "sinh") => Some(Some(g.function("cosh", [a1[0]]))),
                                    ("sec", "cos")
                                    | ("csc", "sin")
                                    | ("tan", "cot")
                                    | ("sech", "cosh")
                                    | ("csch", "sinh")
                                    | ("tanh", "coth") => Some(None),
                                    _ => None,
                                };
                                if let Some(opt_e) = rep {
                                    let (hi, lo) = if i > j { (i, j) } else { (j, i) };
                                    rest.remove(hi);
                                    rest.remove(lo);
                                    if let Some(ne) = opt_e {
                                        rest.push(ne);
                                    }
                                    changed = true;
                                    break 'pair;
                                }
                            }
                            // (a / b) * b -> a
                            if let ExprKind::Div(num, den) = g.get(rest[i]).kind
                                && den == rest[j]
                            {
                                let (hi, lo) = if i > j { (i, j) } else { (j, i) };
                                rest.remove(hi);
                                rest.remove(lo);
                                rest.push(num);
                                changed = true;
                                break 'pair;
                            }
                        }
                    }
                }
                if coeff.is_zero() {
                    return g.integer(0);
                }
                if rest.is_empty() {
                    return crate::poly::expr_from_rational(g, &coeff);
                }
                if coeff == -BigRational::one() && rest.len() == 1 {
                    return g.neg(rest[0]);
                }
                if !coeff.is_one() {
                    rest.insert(0, crate::poly::expr_from_rational(g, &coeff));
                }
                // Collapse numerator factors into a single Div(num, den) factor:
                // e.g. 2 * x * (1 / (x^2 + 1)) -> (2 * x) / (x^2 + 1), (1/2) * (4 / (x^2 + 4)) -> 2 / (x^2 + 4)
                let div_indices: Vec<usize> = rest
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, &r)| {
                        if matches!(g.get(r).kind, ExprKind::Div(_, _)) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect();
                if div_indices.len() == 1 && rest.len() > 1 {
                    let di = div_indices[0];
                    if let ExprKind::Div(d_num, d_den) = g.get(rest[di]).kind {
                        let mut num_factors: Vec<ExprId> = rest
                            .iter()
                            .enumerate()
                            .filter(|&(idx, _)| idx != di)
                            .map(|(_, &r)| r)
                            .collect();
                        if !is_one_id(g, d_num) {
                            num_factors.push(d_num);
                        }
                        let combined_num = if num_factors.is_empty() {
                            g.integer(1)
                        } else if num_factors.len() == 1 {
                            num_factors[0]
                        } else {
                            rec(g, g.mul(num_factors), assumptions)
                        };
                        return g.div(combined_num, d_den);
                    }
                }
                if rest.len() == 1 {
                    rest[0]
                } else {
                    g.mul(rest)
                }
            }
            ExprKind::Div(num, den) => {
                let sn = rec(g, *num, assumptions);
                let sd = rec(g, *den, assumptions);
                if is_zero_id(g, sn) && !is_zero_id(g, sd) {
                    return g.integer(0);
                }
                if is_one_id(g, sd) {
                    return sn;
                }
                if sn == sd {
                    return g.integer(1);
                }
                if let (Some(qn), Some(qd)) = (as_rat(g, sn), as_rat(g, sd))
                    && !qd.is_zero()
                {
                    return crate::poly::expr_from_rational(g, &(qn / qd));
                }
                // sin(u)/cos(u) -> tan(u), cos(u)/sin(u) -> cot(u)
                if let (
                    ExprKind::Function { name: n1, args: a1 },
                    ExprKind::Function { name: n2, args: a2 },
                ) = (&g.get(sn).kind, &g.get(sd).kind)
                    && a1.len() == 1
                    && a2.len() == 1
                    && a1[0] == a2[0]
                {
                    match (
                        g.symbols.resolve(*n1).as_deref(),
                        g.symbols.resolve(*n2).as_deref(),
                    ) {
                        (Some("sin"), Some("cos")) => return g.function("tan", [a1[0]]),
                        (Some("cos"), Some("sin")) => return g.function("cot", [a1[0]]),
                        (Some("sinh"), Some("cosh")) => return g.function("tanh", [a1[0]]),
                        (Some("cosh"), Some("sinh")) => return g.function("coth", [a1[0]]),
                        _ => {}
                    }
                }
                // 1 / (1 / x) => x, a / (b / c) => (a * c) / b
                if let ExprKind::Div(dn, dd) = g.get(sd).kind {
                    let new_num = rec(g, g.mul([sn, dd]), assumptions);
                    return rec(g, g.div(new_num, dn), assumptions);
                }
                if let ExprKind::Div(nn, nd) = g.get(sn).kind {
                    let new_den = rec(g, g.mul([nd, sd]), assumptions);
                    return rec(g, g.div(nn, new_den), assumptions);
                }
                // Factor cancellation between Mul factors and denominator: (2*x)/2 -> x
                if let ExprKind::Mul(fs) = g.get(sn).kind
                    && fs.contains(&sd)
                {
                    let mut removed = false;
                    let mut rem = Vec::new();
                    for &f in &fs {
                        if !removed && f == sd {
                            removed = true;
                        } else {
                            rem.push(f);
                        }
                    }
                    return if rem.is_empty() {
                        g.integer(1)
                    } else if rem.len() == 1 {
                        rem[0]
                    } else {
                        g.mul(rem)
                    };
                }
                // Exact univariate or multivariate polynomial GCD cancellation (e.g. (x^2 - y^2)/(x - y) -> x + y)
                let cancelled = crate::simplify::AlgebraicTransformations::cancel(g, g.div(sn, sd));
                if cancelled != g.div(sn, sd) {
                    return cancelled;
                }
                g.div(sn, sd)
            }
            ExprKind::Pow(base, exp) => {
                let sb = rec(g, *base, assumptions);
                let se = rec(g, *exp, assumptions);
                if is_zero_id(g, se) {
                    return g.integer(1);
                }
                if is_one_id(g, se) {
                    return sb;
                }
                // e^x is exp(x), so that the rules for exp (and exp(i pi) = -1) apply to both spellings.
                let base_is_e = matches!(
                    g.get(sb).kind,
                    ExprKind::Number(algebra_core::Number::Constant(algebra_core::Constant::E))
                ) || matches!(g.get(sb).kind, ExprKind::Symbol(s) if g.symbols.resolve(s).as_deref() == Some("e"));
                if base_is_e {
                    return rec(g, g.function("exp", [se]), assumptions);
                }
                // (x^2)^(1/2) under assumptions
                if as_rat(g, se) == Some(BigRational::new(1.into(), 2.into()))
                    && let ExprKind::Pow(b_in, e_in) = g.get(sb).kind
                    && as_rat(g, e_in) == Some(BigRational::from_integer(2.into()))
                    && let ExprKind::Symbol(sym) = g.get(b_in).kind
                {
                    if assumptions.is(sym, Predicate::Positive)
                        || assumptions.is(sym, Predicate::NonNegative)
                    {
                        return b_in;
                    }
                    if assumptions.is(sym, Predicate::Negative) {
                        return g.neg(b_in);
                    }
                    if assumptions.is(sym, Predicate::Real) {
                        return g.function("abs", [b_in]);
                    }
                }
                // i^2 -> -1 for both Symbol("i") and Constant::I
                let is_i = is_imag_unit(g, sb);
                if is_i
                    && let Some(qe) = as_rat(g, se)
                    && qe.is_integer()
                {
                    let four = num_bigint::BigInt::from(4);
                    let rem4 = (qe.numer() % &four + &four) % &four;
                    if let Some(r) = rem4.to_i64() {
                        return match r {
                            0 => g.integer(1),
                            1 => sb,
                            2 => g.integer(-1),
                            _ => g.neg(sb),
                        };
                    }
                }
                // Euler's identity: e^(i * q * pi)
                if matches!(
                    g.get(sb).kind,
                    ExprKind::Number(Number::Constant(algebra_core::Constant::E))
                ) && let Some(kq) = extract_i_pi_rational(g, se)
                {
                    let two = BigRational::from_integer(2.into());
                    let mut r = kq;
                    if r >= two || r < BigRational::zero() {
                        let fl = (r.clone() / &two).floor();
                        r -= fl * &two;
                    }
                    if r.is_zero() {
                        return g.integer(1);
                    }
                    if r == BigRational::one() {
                        return g.integer(-1);
                    }
                    if r == BigRational::new(1.into(), 2.into()) {
                        return g.constant(algebra_core::Constant::I);
                    }
                    if r == BigRational::new(3.into(), 2.into()) {
                        return g.neg(g.constant(algebra_core::Constant::I));
                    }
                }
                // x^(1/2) radical simplification for integers (sqrt(8) -> 2*sqrt(2))
                if let Some(qe) = as_rat(g, se) {
                    if qe == BigRational::new(1.into(), 2.into())
                        && let Some(r) = simplify_sqrt_int(g, sb)
                    {
                        return r;
                    }
                    if qe.is_integer()
                        && let Some(qb) = as_rat(g, sb)
                        && let Some(e_i32) = qe.numer().to_i32()
                        && e_i32.abs() <= 64
                        && !(qb.is_zero() && e_i32 < 0)
                    {
                        let p = if e_i32 >= 0 {
                            qb.pow(e_i32)
                        } else {
                            qb.recip().pow(-e_i32)
                        };
                        return crate::poly::expr_from_rational(g, &p);
                    }
                }
                g.pow(sb, se)
            }
            ExprKind::Function { name, args } => {
                let fn_name = g.symbols.resolve(*name).unwrap_or_default();
                let sargs: Vec<ExprId> = args.iter().map(|&a| rec(g, a, assumptions)).collect();
                if sargs.len() == 1 {
                    let a = sargs[0];
                    if matches!(fn_name.as_str(), "sin" | "cos" | "tan")
                        && let Some(k_pi) = extract_pi_rational(g, a)
                        && let Some(exact) = eval_exact_trig_pi(g, &fn_name, &k_pi)
                    {
                        return exact;
                    }
                    match fn_name.as_str() {
                        "sin" | "tan" | "sinh" | "tanh" | "asin" | "arcsin" | "atan" | "arctan"
                            if is_zero_id(g, a) =>
                        {
                            return g.integer(0);
                        }
                        "cos" | "cosh" | "exp" if is_zero_id(g, a) => return g.integer(1),
                        // abs(-x) = abs(x), and the odd/even symmetries of the hyperbolic and inverse functions.
                        "abs" if matches!(g.get(a).kind, ExprKind::Neg(_)) => {
                            if let ExprKind::Neg(inner) = g.get(a).kind {
                                return rec(g, g.function("abs", [inner]), assumptions);
                            }
                        }
                        "acos" | "arccos" => {
                            let pi = g.constant(algebra_core::Constant::Pi);
                            if is_one_id(g, a) {
                                return g.integer(0);
                            }
                            if is_zero_id(g, a) {
                                return g.div(pi, g.integer(2));
                            }
                            if let Some(q) = as_rat(g, a) {
                                if q == BigRational::new(1.into(), 2.into()) {
                                    return g.div(pi, g.integer(3));
                                }
                                if q == BigRational::new((-1).into(), 2.into()) {
                                    return g.div(g.mul([g.integer(2), pi]), g.integer(3));
                                }
                                if q == BigRational::from_integer((-1).into()) {
                                    return pi;
                                }
                            }
                        }
                        "asin" | "arcsin" => {
                            let pi = g.constant(algebra_core::Constant::Pi);
                            if is_zero_id(g, a) {
                                return g.integer(0);
                            }
                            if is_one_id(g, a) {
                                return g.div(pi, g.integer(2));
                            }
                            if let Some(q) = as_rat(g, a) {
                                if q == BigRational::new(1.into(), 2.into()) {
                                    return g.div(pi, g.integer(6));
                                }
                                if q == BigRational::new((-1).into(), 2.into()) {
                                    return g.neg(g.div(pi, g.integer(6)));
                                }
                                if q == BigRational::from_integer((-1).into()) {
                                    return g.neg(g.div(pi, g.integer(2)));
                                }
                            }
                        }
                        "atan" | "arctan" if is_one_id(g, a) => {
                            let pi = g.constant(algebra_core::Constant::Pi);
                            return g.div(pi, g.integer(4));
                        }
                        "atan" | "arctan"
                            if as_rat(g, a) == Some(BigRational::from_integer((-1).into())) =>
                        {
                            let pi = g.constant(algebra_core::Constant::Pi);
                            return g.neg(g.div(pi, g.integer(4)));
                        }
                        "ln" | "log" => {
                            let is_euler_e = |id: ExprId| -> bool {
                                matches!(
                                    g.get(id).kind,
                                    ExprKind::Number(Number::Constant(algebra_core::Constant::E))
                                ) || matches!(
                                    g.get(id).kind,
                                    ExprKind::Symbol(sym) if g.symbols.resolve(sym).as_deref() == Some("e")
                                )
                            };
                            if is_one_id(g, a) {
                                return g.integer(0);
                            }
                            if is_euler_e(a) {
                                return g.integer(1);
                            }
                            if let ExprKind::Pow(eb, ee) = g.get(a).kind
                                && is_euler_e(eb)
                                && (!contains_imaginary_unit(g, ee)
                                    || matches!(g.get(ee).kind, ExprKind::Symbol(sym) if assumptions.is(sym, Predicate::Real)))
                            {
                                return ee;
                            }
                            if let ExprKind::Function {
                                name: inner_n,
                                args: inner_a,
                            } = g.get(a).kind
                                && g.symbols.resolve(inner_n).as_deref() == Some("exp")
                                && inner_a.len() == 1
                                && (!contains_imaginary_unit(g, inner_a[0])
                                    || matches!(g.get(inner_a[0]).kind, ExprKind::Symbol(sym) if assumptions.is(sym, Predicate::Real)))
                            {
                                return inner_a[0];
                            }
                        }
                        "exp" => {
                            if let Some(kq) = extract_i_pi_rational(g, a) {
                                let two = BigRational::from_integer(2.into());
                                let mut r = kq;
                                if r >= two || r < BigRational::zero() {
                                    let fl = (r.clone() / &two).floor();
                                    r -= fl * &two;
                                }
                                if r.is_zero() {
                                    return g.integer(1);
                                }
                                if r == BigRational::one() {
                                    return g.integer(-1);
                                }
                                if r == BigRational::new(1.into(), 2.into()) {
                                    return g.constant(algebra_core::Constant::I);
                                }
                                if r == BigRational::new(3.into(), 2.into()) {
                                    return g.neg(g.constant(algebra_core::Constant::I));
                                }
                            }
                            if let ExprKind::Function {
                                name: inner_n,
                                args: inner_a,
                            } = g.get(a).kind
                                && matches!(
                                    g.symbols.resolve(inner_n).as_deref(),
                                    Some("ln" | "log")
                                )
                                && inner_a.len() == 1
                            {
                                return inner_a[0];
                            }
                            // exp(c ln u) = u^c for a rational c, and exp(-ln u) = 1/u.
                            let log_of = |id: ExprId| -> Option<ExprId> {
                                if let ExprKind::Function {
                                    name: inner_n,
                                    args: inner_a,
                                } = g.get(id).kind
                                    && matches!(
                                        g.symbols.resolve(inner_n).as_deref(),
                                        Some("ln" | "log")
                                    )
                                    && inner_a.len() == 1
                                {
                                    Some(inner_a[0])
                                } else {
                                    None
                                }
                            };
                            if let ExprKind::Neg(inner) = g.get(a).kind
                                && let Some(u) = log_of(inner)
                            {
                                return g.pow(u, g.integer(-1));
                            }
                            if let ExprKind::Mul(factors) = g.get(a).kind.clone() {
                                let mut coefficient = BigRational::one();
                                let mut logs = Vec::new();
                                let mut other = false;
                                for &f in factors.iter() {
                                    if let Some(q) = as_rat(g, f) {
                                        coefficient *= q;
                                    } else if let Some(u) = log_of(f) {
                                        logs.push(u);
                                    } else {
                                        other = true;
                                    }
                                }
                                if !other && logs.len() == 1 {
                                    return g.pow(
                                        logs[0],
                                        crate::poly::expr_from_rational(g, &coefficient),
                                    );
                                }
                            }
                        }
                        "abs" => {
                            if let Some(q) = as_rat(g, a) {
                                return crate::poly::expr_from_rational(g, &q.abs());
                            }
                            if let ExprKind::Symbol(sym) = g.get(a).kind {
                                if assumptions.is(sym, Predicate::Positive)
                                    || assumptions.is(sym, Predicate::NonNegative)
                                {
                                    return a;
                                }
                                if assumptions.is(sym, Predicate::Negative) {
                                    return g.neg(a);
                                }
                            }
                        }
                        "sqrt" | "factorial" => {
                            if fn_name == "sqrt" {
                                if let Some(r) = simplify_sqrt_int(g, a) {
                                    return r;
                                }
                                if let ExprKind::Pow(b_in, e_in) = g.get(a).kind
                                    && as_rat(g, e_in) == Some(BigRational::from_integer(2.into()))
                                    && let ExprKind::Symbol(sym) = g.get(b_in).kind
                                {
                                    if assumptions.is(sym, Predicate::Positive)
                                        || assumptions.is(sym, Predicate::NonNegative)
                                    {
                                        return b_in;
                                    }
                                    if assumptions.is(sym, Predicate::Negative) {
                                        return g.neg(b_in);
                                    }
                                    if assumptions.is(sym, Predicate::Real) {
                                        return g.function("abs", [b_in]);
                                    }
                                }
                            }
                            if let Ok(Some(q)) = crate::numeric::try_eval_exact(
                                g,
                                g.function(&fn_name, sargs.clone()),
                                &std::collections::HashMap::new(),
                            ) {
                                return crate::poly::expr_from_rational(g, &q);
                            }
                        }
                        _ => {}
                    }
                } else if sargs.len() == 2
                    && matches!(
                        fn_name.as_str(),
                        "log" | "mod" | "Mod" | "gcd" | "lcm" | "binomial"
                    )
                {
                    if fn_name == "log" && is_one_id(g, sargs[0]) {
                        return g.integer(0);
                    }
                    if fn_name == "log" && sargs[0] == sargs[1] {
                        return g.integer(1);
                    }
                    if let Ok(Some(q)) = crate::numeric::try_eval_exact(
                        g,
                        g.function(&fn_name, sargs.clone()),
                        &std::collections::HashMap::new(),
                    ) {
                        return crate::poly::expr_from_rational(g, &q);
                    }
                }
                g.function(&fn_name, sargs)
            }
            _ => e,
        }
    }

    let _ = BigRational::zero().denom().gcd(&1.into());
    rec(graph, id, assumptions)
}

fn post_simplify_algebraic_with_assumptions(
    graph: &ExprGraph,
    id: ExprId,
    assumptions: &crate::logic::AssumptionsContext,
) -> ExprId {
    fn ast_size(g: &ExprGraph, e: ExprId) -> usize {
        match &g.get(e).kind {
            ExprKind::Add(xs) | ExprKind::Mul(xs) => {
                1 + xs.iter().map(|&x| ast_size(g, x)).sum::<usize>()
            }
            ExprKind::Sub(a, b) | ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                1 + ast_size(g, *a) + ast_size(g, *b)
            }
            ExprKind::Neg(a) => 1 + ast_size(g, *a),
            ExprKind::Function { args, .. } => {
                1 + args.iter().map(|&x| ast_size(g, x)).sum::<usize>()
            }
            _ => 1,
        }
    }

    let cleaned = pre_simplify_algebraic_with_assumptions(graph, id, assumptions);
    let expanded = crate::simplify::AlgebraicTransformations::expand(graph, cleaned);
    if ast_size(graph, expanded) < ast_size(graph, cleaned) {
        expanded
    } else {
        cleaned
    }
}

/// Simplification extension trait for `ExprGraph`.
pub trait SymbolicSimplifier {
    /// Simplify `root` expression DAG using non-greedy E-Graph rewrite rules.
    fn simplify(&self, root: ExprId) -> ExprId;
}

impl SymbolicSimplifier for ExprGraph {
    fn simplify(&self, root: ExprId) -> ExprId {
        match Simplifier::simplify(self, root) {
            Ok(simp) => simp,
            Err(_) => root,
        }
    }
}
