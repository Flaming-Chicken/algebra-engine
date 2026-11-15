//! Strangler-Fig bridge implementation for `algebra-core` expressions.

use crate::context::LegacyBridgeContext;
use crate::traits::RetrofittableAst;
use algebra_core::domain::Domain;
use algebra_core::expr::{ExprKind, ExprNode};
use algebra_core::graph::ExprGraph;
use algebra_core::id::{ExprId, SymbolId};
use algebra_core::number::{Constant, Number};
use cat_core::{
    CatRuntimeContext, CatTerm, DiagramId, DimVector, EnrichedSpine, HyperEdge, HyperOp,
    ObjectDescriptor, OpenHypergraph, ScalarAtom, ScalarId, ScalarLattice,
};
use num_traits::ToPrimitive;
use smallvec::SmallVec;
use thiserror::Error;

/// Errors arising during AST lifting and lowering.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum BridgeError {
    #[error("Missing symbol name for ID {0:?}")]
    MissingSymbol(SymbolId),
    #[error("Invalid dimensional structure: {0}")]
    DimensionalMismatch(String),
    #[error("Internal hypergraph lowering error: {0}")]
    LoweringError(String),
}

/// Recursive AST tree representation mirroring legacy `algebra-core` expressions,
/// providing seamless conversions with [`ExprGraph`] and implementing [`RetrofittableAst`].
#[derive(Clone, Debug, PartialEq)]
pub enum LegacyAstTree {
    Number(Number),
    Symbol(String),
    Add(Vec<LegacyAstTree>),
    Mul(Vec<LegacyAstTree>),
    Pow(Box<LegacyAstTree>, Box<LegacyAstTree>),
    Sub(Box<LegacyAstTree>, Box<LegacyAstTree>),
    Div(Box<LegacyAstTree>, Box<LegacyAstTree>),
    Neg(Box<LegacyAstTree>),
    Derivative {
        expr: Box<LegacyAstTree>,
        wrt: String,
        order: u32,
    },
    Matrix {
        rows: usize,
        cols: usize,
        elements: Vec<LegacyAstTree>,
    },
    TensorContraction {
        tensor_a: Box<LegacyAstTree>,
        tensor_b: Box<LegacyAstTree>,
        contracted_indices: Vec<(String, String)>,
    },
    Function {
        name: String,
        args: Vec<LegacyAstTree>,
    },
    Surreal(cat_core::scalar::SurrealAtom),
    Adele(cat_core::scalar::AdeleAtom),
}

impl LegacyAstTree {
    /// Extract a full recursive tree from an [`ExprGraph`] starting at [`ExprId`].
    pub fn from_graph(id: ExprId, graph: &ExprGraph) -> Result<Self, BridgeError> {
        let node = graph.get(id);
        match node.kind {
            ExprKind::Number(n) => Ok(Self::Number(n)),
            ExprKind::Symbol(s) => {
                let name = graph
                    .symbols
                    .resolve(s)
                    .ok_or(BridgeError::MissingSymbol(s))?;
                Ok(Self::Symbol(name))
            }
            ExprKind::Add(terms) => {
                let mut v = Vec::with_capacity(terms.len());
                for &t in &terms {
                    v.push(Self::from_graph(t, graph)?);
                }
                Ok(Self::Add(v))
            }
            ExprKind::Mul(factors) => {
                let mut v = Vec::with_capacity(factors.len());
                for &f in &factors {
                    v.push(Self::from_graph(f, graph)?);
                }
                Ok(Self::Mul(v))
            }
            ExprKind::Pow(base, exp) => Ok(Self::Pow(
                Box::new(Self::from_graph(base, graph)?),
                Box::new(Self::from_graph(exp, graph)?),
            )),
            ExprKind::Sub(lhs, rhs) => Ok(Self::Sub(
                Box::new(Self::from_graph(lhs, graph)?),
                Box::new(Self::from_graph(rhs, graph)?),
            )),
            ExprKind::Div(num, den) => Ok(Self::Div(
                Box::new(Self::from_graph(num, graph)?),
                Box::new(Self::from_graph(den, graph)?),
            )),
            ExprKind::Neg(inner) => Ok(Self::Neg(Box::new(Self::from_graph(inner, graph)?))),
            ExprKind::Derivative { expr, wrt, order } => {
                let wrt_name = graph
                    .symbols
                    .resolve(wrt)
                    .ok_or(BridgeError::MissingSymbol(wrt))?;
                Ok(Self::Derivative {
                    expr: Box::new(Self::from_graph(expr, graph)?),
                    wrt: wrt_name,
                    order,
                })
            }
            ExprKind::Matrix {
                rows,
                cols,
                elements,
            } => {
                let mut v = Vec::with_capacity(elements.len());
                for &e in &elements {
                    v.push(Self::from_graph(e, graph)?);
                }
                Ok(Self::Matrix {
                    rows,
                    cols,
                    elements: v,
                })
            }
            ExprKind::TensorContraction {
                tensor_a,
                tensor_b,
                contracted_indices,
            } => {
                let mut c_indices = Vec::with_capacity(contracted_indices.len());
                for (s1, s2) in contracted_indices {
                    let n1 = graph
                        .symbols
                        .resolve(s1)
                        .ok_or(BridgeError::MissingSymbol(s1))?;
                    let n2 = graph
                        .symbols
                        .resolve(s2)
                        .ok_or(BridgeError::MissingSymbol(s2))?;
                    c_indices.push((n1, n2));
                }
                Ok(Self::TensorContraction {
                    tensor_a: Box::new(Self::from_graph(tensor_a, graph)?),
                    tensor_b: Box::new(Self::from_graph(tensor_b, graph)?),
                    contracted_indices: c_indices,
                })
            }
            ExprKind::Function { name, args } => {
                let f_name = graph
                    .symbols
                    .resolve(name)
                    .ok_or(BridgeError::MissingSymbol(name))?;
                let mut v = Vec::with_capacity(args.len());
                for &a in &args {
                    v.push(Self::from_graph(a, graph)?);
                }
                Ok(Self::Function {
                    name: f_name,
                    args: v,
                })
            }
            _ => Err(BridgeError::LoweringError(
                "Unsupported ExprKind variant".into(),
            )),
        }
    }

    /// Reconstruct an [`ExprId`] in the destination [`ExprGraph`].
    pub fn to_graph(&self, graph: &ExprGraph) -> ExprId {
        match self {
            Self::Number(n) => graph.number(n.clone()),
            Self::Symbol(s) => graph.symbol(s),
            Self::Add(terms) => {
                let ids: Vec<ExprId> = terms.iter().map(|t| t.to_graph(graph)).collect();
                graph.add(ids)
            }
            Self::Mul(factors) => {
                let ids: Vec<ExprId> = factors.iter().map(|f| f.to_graph(graph)).collect();
                graph.mul(ids)
            }
            Self::Pow(base, exp) => {
                let b = base.to_graph(graph);
                let e = exp.to_graph(graph);
                graph.pow(b, e)
            }
            Self::Sub(lhs, rhs) => {
                let l = lhs.to_graph(graph);
                let r = rhs.to_graph(graph);
                graph.sub(l, r)
            }
            Self::Div(num, den) => {
                let n = num.to_graph(graph);
                let d = den.to_graph(graph);
                graph.div(n, d)
            }
            Self::Neg(inner) => {
                let i = inner.to_graph(graph);
                graph.neg(i)
            }
            Self::Derivative { expr, wrt, order } => {
                let e = expr.to_graph(graph);
                let w = graph.symbols.get_or_intern(wrt);
                graph.intern(ExprNode::new(
                    Domain::Reals,
                    ExprKind::Derivative {
                        expr: e,
                        wrt: w,
                        order: *order,
                    },
                ))
            }
            Self::Matrix {
                rows,
                cols,
                elements,
            } => {
                let ids: Vec<ExprId> = elements.iter().map(|e| e.to_graph(graph)).collect();
                graph.intern(ExprNode::new(
                    Domain::Reals,
                    ExprKind::Matrix {
                        rows: *rows,
                        cols: *cols,
                        elements: ids,
                    },
                ))
            }
            Self::TensorContraction {
                tensor_a,
                tensor_b,
                contracted_indices,
            } => {
                let ta = tensor_a.to_graph(graph);
                let tb = tensor_b.to_graph(graph);
                let indices = contracted_indices
                    .iter()
                    .map(|(s1, s2)| {
                        (
                            graph.symbols.get_or_intern(s1),
                            graph.symbols.get_or_intern(s2),
                        )
                    })
                    .collect();
                graph.intern(ExprNode::new(
                    Domain::Reals,
                    ExprKind::TensorContraction {
                        tensor_a: ta,
                        tensor_b: tb,
                        contracted_indices: indices,
                    },
                ))
            }
            Self::Function { name, args } => {
                let f = graph.symbols.get_or_intern(name);
                let a = args.iter().map(|x| x.to_graph(graph)).collect();
                graph.intern(ExprNode::new(
                    Domain::Reals,
                    ExprKind::Function { name: f, args: a },
                ))
            }
            Self::Surreal(s) => match s {
                cat_core::scalar::SurrealAtom::Zero => graph.number(Number::Integer(0)),
                cat_core::scalar::SurrealAtom::Integer(i) => graph.number(Number::Integer(*i)),
                _ => graph.symbol("surreal_atom"),
            },
            Self::Adele(a) => graph.number(Number::rational(a.arch_num, a.arch_den)),
        }
    }
}

impl RetrofittableAst for LegacyAstTree {
    type LegacyError = BridgeError;

    fn lift_to_cat(
        &self,
        ctx: &mut CatRuntimeContext,
        bridge: &mut LegacyBridgeContext,
    ) -> Result<CatTerm, Self::LegacyError> {
        let dim_scalar = ctx.intern_obj(ObjectDescriptor::DIMENSIONLESS_SCALAR);

        match self {
            // Scalar numbers
            Self::Number(num) => {
                let atom = match num {
                    Number::Integer(i) => ScalarAtom::Integer(*i),
                    Number::BigInteger(bi) => {
                        if let Some(i) = bi.to_i64() {
                            ScalarAtom::Integer(i)
                        } else {
                            ScalarAtom::Float(bi.to_f64().unwrap_or(f64::INFINITY))
                        }
                    }
                    Number::Rational(n, d) => ScalarAtom::rational(*n, *d),
                    Number::BigRational(br) => {
                        if let (Some(n), Some(d)) = (br.numer().to_i64(), br.denom().to_i64()) {
                            ScalarAtom::rational(n, d)
                        } else {
                            ScalarAtom::Float(br.to_f64().unwrap_or(0.0))
                        }
                    }
                    Number::Constant(Constant::I) => ScalarAtom::imaginary_unit(),
                    Number::Constant(Constant::Pi) => ScalarAtom::Float(std::f64::consts::PI),
                    Number::Constant(Constant::E) => ScalarAtom::Float(std::f64::consts::E),
                    Number::Float(u) => ScalarAtom::Float(f64::from_bits(*u)),
                    _ => ScalarAtom::Float(f64::NAN),
                };
                let scalar_id = ctx.intern_scalar(atom.clone());
                let spine = EnrichedSpine::exact(scalar_id, atom);
                let empty_diagram = ctx.intern_diagram(OpenHypergraph::new());
                Ok(CatTerm::new(dim_scalar, spine, empty_diagram))
            }

            // Named symbol variable
            Self::Symbol(name) => {
                let sym_id = bridge.get_or_register_symbol(name);
                let mut diag = OpenHypergraph::new();
                let w = diag.alloc_wire();
                diag.add_input_port(w, dim_scalar);
                diag.add_output_port(w, dim_scalar);
                diag.add_edge(HyperEdge::unary(w, w, HyperOp::External(sym_id)));

                let diagram_id = ctx.intern_diagram(diag);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(dim_scalar, spine, diagram_id))
            }

            // Surreal Numbers
            Self::Surreal(s) => {
                let atom = ScalarAtom::Surreal(s.clone());
                let sid = ctx.intern_scalar(atom.clone());
                let spine = EnrichedSpine::exact(sid, atom);
                let empty_diag = ctx.intern_diagram(OpenHypergraph::new());
                Ok(CatTerm::new(dim_scalar, spine, empty_diag))
            }

            // Adele Numbers
            Self::Adele(a) => {
                let atom = ScalarAtom::Adele(a.clone());
                let sid = ctx.intern_scalar(atom.clone());
                let spine = EnrichedSpine::exact(sid, atom);
                let empty_diag = ctx.intern_diagram(OpenHypergraph::new());
                Ok(CatTerm::new(dim_scalar, spine, empty_diag))
            }

            // Polyadic addition: check for complex number pattern re + im * i
            Self::Add(terms) => {
                // Check if this is a complex number: re + im * I
                if terms.len() == 2 {
                    if let (re_term, Self::Mul(im_factors)) = (&terms[0], &terms[1]) {
                        if im_factors.len() == 2 {
                            if let (im_term, Self::Number(Number::Constant(Constant::I))) =
                                (&im_factors[0], &im_factors[1])
                            {
                                if let (Self::Number(re_num), Self::Number(im_num)) =
                                    (re_term, im_term)
                                {
                                    if let (Number::Integer(r), Number::Integer(i)) =
                                        (re_num, im_num)
                                    {
                                        let atom = ScalarAtom::gaussian_int(*r, *i);
                                        let sid = ctx.intern_scalar(atom.clone());
                                        let spine = EnrichedSpine::exact(sid, atom);
                                        let diag_id = ctx.intern_diagram(OpenHypergraph::new());
                                        return Ok(CatTerm::new(dim_scalar, spine, diag_id));
                                    } else if let (
                                        Number::Rational(rn, rd),
                                        Number::Rational(in_val, id_val),
                                    ) = (re_num, im_num)
                                    {
                                        let atom = ScalarAtom::complex_rational(
                                            *rn, *rd, *in_val, *id_val,
                                        );
                                        let sid = ctx.intern_scalar(atom.clone());
                                        let spine = EnrichedSpine::exact(sid, atom);
                                        let diag_id = ctx.intern_diagram(OpenHypergraph::new());
                                        return Ok(CatTerm::new(dim_scalar, spine, diag_id));
                                    }
                                }
                            }
                        }
                    }
                }

                // General addition: lift child terms and combine in hypergraph
                let mut child_terms = Vec::with_capacity(terms.len());
                let mut total_scalar = ScalarAtom::Zero;

                for t in terms {
                    let cat = t.lift_to_cat(ctx, bridge)?;
                    if let Some(atom) = ctx.get_scalar(cat.spine.scalar) {
                        if cat.diagram == DiagramId(0) {
                            total_scalar = ScalarLattice::add(&total_scalar, &atom);
                            continue;
                        }
                    }
                    child_terms.push(cat);
                }

                if child_terms.is_empty() {
                    let sid = ctx.intern_scalar(total_scalar.clone());
                    let spine = EnrichedSpine::exact(sid, total_scalar);
                    let empty_diag = ctx.intern_diagram(OpenHypergraph::new());
                    return Ok(CatTerm::new(dim_scalar, spine, empty_diag));
                }

                let mut combined_diag = OpenHypergraph::new();
                let out_w = combined_diag.alloc_wire();
                combined_diag.add_output_port(out_w, dim_scalar);

                let mut in_wires = SmallVec::new();
                for child in &child_terms {
                    let child_diag = ctx.get_diagram(child.diagram).unwrap_or_default();
                    let child_tensor = combined_diag.tensor(&child_diag);
                    combined_diag = child_tensor;
                    if let Some(p) = combined_diag.boundary_ports.last() {
                        in_wires.push(p.wire);
                    }
                }

                combined_diag.add_edge(HyperEdge::new(
                    in_wires,
                    smallvec::smallvec![out_w],
                    HyperOp::Add,
                ));

                let diag_id = ctx.intern_diagram(combined_diag);
                let sid = ctx.intern_scalar(total_scalar.clone());
                let spine = EnrichedSpine::exact(sid, total_scalar);
                Ok(CatTerm::new(dim_scalar, spine, diag_id))
            }

            // Multiplication: check for scalar * symbol or im * I
            Self::Mul(factors) => {
                if factors.len() == 2 {
                    if let (
                        Self::Number(Number::Integer(i)),
                        Self::Number(Number::Constant(Constant::I)),
                    ) = (&factors[0], &factors[1])
                    {
                        let atom = ScalarAtom::gaussian_int(0, *i);
                        let sid = ctx.intern_scalar(atom.clone());
                        let spine = EnrichedSpine::exact(sid, atom);
                        let diag_id = ctx.intern_diagram(OpenHypergraph::new());
                        return Ok(CatTerm::new(dim_scalar, spine, diag_id));
                    }
                }

                let mut factor_scalar = ScalarAtom::One;
                let mut non_scalar_terms = Vec::new();

                for f in factors {
                    let cat = f.lift_to_cat(ctx, bridge)?;
                    if let Some(atom) = ctx.get_scalar(cat.spine.scalar) {
                        if cat.diagram == DiagramId(0) {
                            factor_scalar = ScalarLattice::mul(&factor_scalar, &atom);
                            continue;
                        }
                    }
                    non_scalar_terms.push(cat);
                }

                if non_scalar_terms.is_empty() {
                    let sid = ctx.intern_scalar(factor_scalar.clone());
                    let spine = EnrichedSpine::exact(sid, factor_scalar);
                    let empty_diag = ctx.intern_diagram(OpenHypergraph::new());
                    return Ok(CatTerm::new(dim_scalar, spine, empty_diag));
                }

                let mut combined_diag = OpenHypergraph::new();
                let out_w = combined_diag.alloc_wire();
                combined_diag.add_output_port(out_w, dim_scalar);

                let mut in_wires = SmallVec::new();
                for child in &non_scalar_terms {
                    let child_diag = ctx.get_diagram(child.diagram).unwrap_or_default();
                    combined_diag = combined_diag.tensor(&child_diag);
                    if let Some(p) = combined_diag.boundary_ports.last() {
                        in_wires.push(p.wire);
                    }
                }

                combined_diag.add_edge(HyperEdge::new(
                    in_wires,
                    smallvec::smallvec![out_w],
                    HyperOp::Mul,
                ));

                let diag_id = ctx.intern_diagram(combined_diag);
                let sid = ctx.intern_scalar(factor_scalar.clone());
                let spine = EnrichedSpine::exact(sid, factor_scalar);
                Ok(CatTerm::new(dim_scalar, spine, diag_id))
            }

            // Tensor Contraction: erase named dummy indices into anonymous WireId links!
            Self::TensorContraction {
                tensor_a,
                tensor_b,
                contracted_indices,
            } => {
                let cat_a = tensor_a.lift_to_cat(ctx, bridge)?;
                let cat_b = tensor_b.lift_to_cat(ctx, bridge)?;

                let diag_a = ctx.get_diagram(cat_a.diagram).unwrap_or_default();
                let diag_b = ctx.get_diagram(cat_b.diagram).unwrap_or_default();

                let mut merged = diag_a.tensor(&diag_b);

                // Fuse contracted indices into internal wire pairs
                let mut wire_pairs = SmallVec::new();
                for (idx_a, idx_b) in contracted_indices {
                    let w_a = merged.alloc_wire();
                    let w_b = merged.alloc_wire();
                    wire_pairs.push((w_a, w_b));
                    bridge.audit_assumption(format!(
                        "Erased named dummy index pair ({}, {}) into WireId({:?}, {:?})",
                        idx_a, idx_b, w_a, w_b
                    ));
                }

                merged.add_edge(HyperEdge {
                    inputs: wire_pairs.iter().map(|&(w1, _)| w1).collect(),
                    outputs: wire_pairs.iter().map(|&(_, w2)| w2).collect(),
                    op: HyperOp::Contraction { pairs: wire_pairs },
                });

                let diag_id = ctx.intern_diagram(merged);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(cat_a.sort, spine, diag_id))
            }

            // Power
            Self::Pow(base, exp) => {
                let cat_base = base.lift_to_cat(ctx, bridge)?;
                let cat_exp = exp.lift_to_cat(ctx, bridge)?;
                let diag_base = ctx.get_diagram(cat_base.diagram).unwrap_or_default();
                let diag_exp = ctx.get_diagram(cat_exp.diagram).unwrap_or_default();

                let mut combined = diag_base.tensor(&diag_exp);
                let out_w = combined.alloc_wire();
                combined.add_output_port(out_w, dim_scalar);

                let diag_id = ctx.intern_diagram(combined);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(cat_base.sort, spine, diag_id))
            }

            // Subtraction
            Self::Sub(lhs, rhs) => {
                let neg_rhs = Self::Neg(rhs.clone());
                Self::Add(vec![(**lhs).clone(), neg_rhs]).lift_to_cat(ctx, bridge)
            }

            // Division
            Self::Div(num, den) => {
                let cat_num = num.lift_to_cat(ctx, bridge)?;
                let cat_den = den.lift_to_cat(ctx, bridge)?;
                let mut combined = ctx
                    .get_diagram(cat_num.diagram)
                    .unwrap_or_default()
                    .tensor(&ctx.get_diagram(cat_den.diagram).unwrap_or_default());
                let out_w = combined.alloc_wire();
                combined.add_output_port(out_w, dim_scalar);
                combined.add_edge(HyperEdge::unary(out_w, out_w, HyperOp::Inv));

                let diag_id = ctx.intern_diagram(combined);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(cat_num.sort, spine, diag_id))
            }

            // Negation
            Self::Neg(inner) => {
                let cat_inner = inner.lift_to_cat(ctx, bridge)?;
                if let Some(atom) = ctx.get_scalar(cat_inner.spine.scalar) {
                    let neg_atom = ScalarLattice::mul(&ScalarAtom::Integer(-1), &atom);
                    let sid = ctx.intern_scalar(neg_atom.clone());
                    let spine = EnrichedSpine::exact(sid, neg_atom);
                    return Ok(CatTerm::new(cat_inner.sort, spine, cat_inner.diagram));
                }
                Ok(cat_inner)
            }

            // Derivatives
            Self::Derivative { expr, wrt, order } => {
                let cat_expr = expr.lift_to_cat(ctx, bridge)?;
                let mut diag = ctx.get_diagram(cat_expr.diagram).unwrap_or_default();
                let _wrt_sym = bridge.get_or_register_symbol(wrt);
                let w = diag.alloc_wire();
                diag.add_edge(HyperEdge::unary(
                    w,
                    w,
                    HyperOp::Derivation {
                        wrt: w,
                        order: *order,
                    },
                ));

                let diag_id = ctx.intern_diagram(diag);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(cat_expr.sort, spine, diag_id))
            }

            // Matrices
            Self::Matrix {
                rows,
                cols,
                elements,
            } => {
                let mut cat_elements = Vec::with_capacity(elements.len());
                for e in elements {
                    cat_elements.push(e.lift_to_cat(ctx, bridge)?);
                }
                let mat_desc = ObjectDescriptor::homogeneous_matrix(
                    *rows as u32,
                    *cols as u32,
                    DimVector::DIMENSIONLESS,
                );
                let mat_obj = ctx.intern_obj(mat_desc);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                let empty_diag = ctx.intern_diagram(OpenHypergraph::new());
                Ok(CatTerm::new(mat_obj, spine, empty_diag))
            }

            // Function
            Self::Function { name, args } => {
                let mut arg_terms = Vec::with_capacity(args.len());
                for a in args {
                    arg_terms.push(a.lift_to_cat(ctx, bridge)?);
                }
                let fn_sym = bridge.get_or_register_symbol(name);
                let mut diag = OpenHypergraph::new();
                let out_w = diag.alloc_wire();
                diag.add_output_port(out_w, dim_scalar);
                diag.add_edge(HyperEdge::unary(out_w, out_w, HyperOp::External(fn_sym)));

                let diag_id = ctx.intern_diagram(diag);
                let spine = EnrichedSpine::exact(ScalarId(1), ScalarAtom::One);
                Ok(CatTerm::new(dim_scalar, spine, diag_id))
            }
        }
    }

    fn lower_from_cat(
        term: &CatTerm,
        ctx: &CatRuntimeContext,
        bridge: &LegacyBridgeContext,
    ) -> Result<Self, Self::LegacyError> {
        let scalar_atom = ctx
            .get_scalar(term.spine.scalar)
            .unwrap_or(ScalarAtom::Zero);
        let diag = ctx.get_diagram(term.diagram).unwrap_or_default();

        // 1. If hypergraph is empty, reconstruct scalar directly
        if diag.edges.is_empty() {
            let num = match &scalar_atom {
                ScalarAtom::Zero => Number::Integer(0),
                ScalarAtom::One => Number::Integer(1),
                ScalarAtom::Integer(i) => Number::Integer(*i),
                ScalarAtom::Rational(n, d) => Number::rational(*n, *d),
                ScalarAtom::Float(f) => Number::Float(f.to_bits()),
                ScalarAtom::GaussianInt { re, im } => {
                    // Lower to re + im * i
                    if *im == 0 {
                        return Ok(Self::Number(Number::Integer(*re)));
                    } else if *re == 0 {
                        return Ok(Self::Mul(vec![
                            Self::Number(Number::Integer(*im)),
                            Self::Number(Number::Constant(Constant::I)),
                        ]));
                    } else {
                        return Ok(Self::Add(vec![
                            Self::Number(Number::Integer(*re)),
                            Self::Mul(vec![
                                Self::Number(Number::Integer(*im)),
                                Self::Number(Number::Constant(Constant::I)),
                            ]),
                        ]));
                    }
                }
                ScalarAtom::ComplexRational {
                    re_num,
                    re_den,
                    im_num,
                    im_den,
                } => {
                    return Ok(Self::Add(vec![
                        Self::Number(Number::rational(*re_num, *re_den)),
                        Self::Mul(vec![
                            Self::Number(Number::rational(*im_num, *im_den)),
                            Self::Number(Number::Constant(Constant::I)),
                        ]),
                    ]));
                }
                ScalarAtom::ComplexF64 { re, im } => {
                    return Ok(Self::Add(vec![
                        Self::Number(Number::Float(re.to_bits())),
                        Self::Mul(vec![
                            Self::Number(Number::Float(im.to_bits())),
                            Self::Number(Number::Constant(Constant::I)),
                        ]),
                    ]));
                }
                ScalarAtom::Surreal(s) => return Ok(Self::Surreal(s.clone())),
                ScalarAtom::Adele(a) => return Ok(Self::Adele(a.clone())),
                _ => Number::Float(0.0f64.to_bits()),
            };
            return Ok(Self::Number(num));
        }

        // 2. If hypergraph has an external symbol reference (e.g. variable x)
        if diag.edges.len() == 1 {
            if let HyperOp::External(sym_id) = diag.edges[0].op {
                if let Some(name) = bridge.get_symbol_name(sym_id) {
                    let base_sym = Self::Symbol(name.to_string());
                    if !scalar_atom.is_one() && !scalar_atom.is_zero() {
                        let s_node = Self::Number(match scalar_atom {
                            ScalarAtom::Integer(i) => Number::Integer(i),
                            ScalarAtom::Rational(n, d) => Number::rational(n, d),
                            _ => Number::Integer(1),
                        });
                        return Ok(Self::Mul(vec![s_node, base_sym]));
                    }
                    return Ok(base_sym);
                }
            }
        }

        // 3. For hypergraphs with internal contracted wires, synthesize canonical dummy indices _w0, _w1, ...
        for edge in &diag.edges {
            if let HyperOp::Contraction { pairs } = &edge.op {
                let contracted = pairs
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| (format!("_w{}", idx * 2), format!("_w{}", idx * 2 + 1)))
                    .collect();
                return Ok(Self::TensorContraction {
                    tensor_a: Box::new(Self::Symbol("T_a".into())),
                    tensor_b: Box::new(Self::Symbol("T_b".into())),
                    contracted_indices: contracted,
                });
            }
        }

        // 4. Default structural projection
        Ok(Self::Symbol("_canonical_hypergraph".into()))
    }
}
