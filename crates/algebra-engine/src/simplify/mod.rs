//! E-Graph pattern matching and algebraic simplification engine using `egg`.
//!
//! Provides non-greedy equivalence graph simplification over symbolic expressions,
//! including associative-commutative rings, matrix identities, and hypercomplex (quaternion) identities.

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

pub mod bridges;
pub mod detector;
pub mod dynamic;
pub mod probabilistic;
pub mod transformations;

pub use bridges::{
    AskeySchemeGroup, BridgeRouter, EulerBridgeGroup, HyperbolicIdentityGroup, IdentityGroup,
    ItoNilpotentBridgeGroup, LieAlgebraGroup, TransformClusterGroup, TrigIdentityGroup,
};
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
        let node = graph.get(root);
        if let ExprKind::Relational { op, lhs, rhs } = &node.kind {
            let simp_lhs = Self::simplify_with_goal(graph, *lhs, goal).unwrap_or(*lhs);
            let simp_rhs = Self::simplify_with_goal(graph, *rhs, goal).unwrap_or(*rhs);
            return Ok(graph.relational(*op, simp_lhs, simp_rhs));
        }

        tracing::debug!(root_id = ?root, goal = ?goal, "Starting E-Graph simplification");
        let mut rec_expr = RecExpr::default();
        let _egg_root = Self::expr_to_egg(graph, root, &mut rec_expr)?;

        let (rules, _features) = DynamicRuleSelector::select_rules(graph, root, goal);

        let runner = Self::bounded_runner(&rec_expr).run(&rules);

        let extractor = egg::Extractor::new(&runner.egraph, AlgebraicCostFunction);
        let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);
        tracing::info!(best_cost = ?best_cost, "E-Graph saturation and cost extraction completed");

        Self::egg_to_expr(graph, &best_expr)
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
            ExprKind::Symbol(sym) => {
                let name = graph
                    .symbols
                    .resolve(*sym)
                    .unwrap_or_else(|| "x".to_string());
                match name.as_str() {
                    "I" => Ok(rec.add(SymbolicLang::IdentityMatrix)),
                    "i" | "qi" => Ok(rec.add(SymbolicLang::QuaternionI)),
                    "j" | "qj" => Ok(rec.add(SymbolicLang::QuaternionJ)),
                    "k" | "qk" => Ok(rec.add(SymbolicLang::QuaternionK)),
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
            SymbolicLang::Symbol(s) => Ok(graph.symbol(s)),
            SymbolicLang::IdentityMatrix => Ok(graph.symbol("I")),
            SymbolicLang::QuaternionI => Ok(graph.symbol("i")),
            SymbolicLang::QuaternionJ => Ok(graph.symbol("j")),
            SymbolicLang::QuaternionK => Ok(graph.symbol("k")),
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
            _ => Err(SimplifyError::Custom("Unsupported E-graph node".into())),
        }
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
