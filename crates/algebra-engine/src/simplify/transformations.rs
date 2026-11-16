//! Algebraic Format Transformations (`expand`, `collect`, `together`, `cancel`).

use algebra_core::{ExprGraph, ExprId, ExprKind, SymbolId};

/// Algebraic Format Transformer.
pub struct AlgebraicTransformations;

impl AlgebraicTransformations {
    /// Expand products and polynomial powers: (a + b)(c + d) -> a*c + a*d + b*c + b*d, (x + 1)^5 -> ...
    pub fn expand(graph: &ExprGraph, expr: ExprId) -> ExprId {
        tracing::debug!(expr = ?expr, "Performing algebraic expansion");
        if let Some(poly) = mv_poly_from_expr(graph, expr) {
            return mv_poly_to_expr(graph, &poly);
        }
        expand_structural(graph, expr)
    }

    /// Collect terms with respect to target symbol $x$ across all powers: a*x^2 + b*x^2 + c*x + d -> (a + b)*x^2 + c*x + d.
    pub fn collect(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> ExprId {
        use num_traits::{One as _, Zero as _};
        tracing::debug!(expr = ?expr, target = ?target, "Performing term collection");
        let target_sym = graph.symbol(&graph.symbols.resolve(target).unwrap_or_default());

        if let Some(poly) = mv_poly_from_expr(graph, expr) {
            let mut by_deg: std::collections::BTreeMap<u32, MvPoly> =
                std::collections::BTreeMap::new();
            for (mono, coeff) in poly {
                let mut deg = 0u32;
                let mut rest = Vec::new();
                for (atom, e) in mono {
                    if atom == target_sym {
                        deg += e;
                    } else {
                        rest.push((atom, e));
                    }
                }
                let sub = by_deg.entry(deg).or_default();
                let entry = sub
                    .entry(rest.clone())
                    .or_insert_with(num_rational::BigRational::zero);
                *entry += coeff;
                if entry.is_zero() {
                    sub.remove(&rest);
                }
            }

            let mut acc_terms: Vec<ExprId> = Vec::new();
            for (&deg, coeff_poly) in by_deg.iter().rev() {
                if coeff_poly.is_empty() {
                    continue;
                }
                let coeff_expr = mv_poly_to_expr(graph, coeff_poly);
                let term = if deg == 0 {
                    coeff_expr
                } else {
                    let pow_node = if deg == 1 {
                        target_sym
                    } else {
                        graph.pow(target_sym, graph.integer(deg as i64))
                    };
                    if coeff_poly.len() == 1
                        && coeff_poly.get(&Vec::new()).is_some_and(|c| c.is_one())
                    {
                        pow_node
                    } else if coeff_poly.len() == 1
                        && coeff_poly
                            .get(&Vec::new())
                            .is_some_and(|c| *c == -num_rational::BigRational::one())
                    {
                        graph.neg(pow_node)
                    } else {
                        graph.mul([coeff_expr, pow_node])
                    }
                };
                acc_terms.push(term);
            }
            if acc_terms.is_empty() {
                return graph.integer(0);
            }
            return acc_terms
                .into_iter()
                .reduce(|a, b| graph.add([a, b]))
                .unwrap_or_else(|| graph.integer(0));
        }

        // General non-polynomial fallback: flatten Add/Sub/Neg and group by integer power of target_sym
        let mut flat = Vec::new();
        flatten_signed_terms(graph, expr, false, &mut flat);
        let mut groups: std::collections::BTreeMap<u32, Vec<ExprId>> =
            std::collections::BTreeMap::new();
        for (term, neg) in flat {
            let (deg, coeff) = extract_target_power(graph, term, target_sym);
            let signed_coeff = if neg { graph.neg(coeff) } else { coeff };
            groups.entry(deg).or_default().push(signed_coeff);
        }
        let mut acc_terms = Vec::new();
        for (deg, cs) in groups.into_iter().rev() {
            let sum_c = if cs.len() == 1 { cs[0] } else { graph.add(cs) };
            let simp_c = crate::Simplifier::simplify(graph, sum_c).unwrap_or(sum_c);
            if matches!(
                graph.get(simp_c).kind,
                ExprKind::Number(algebra_core::Number::Integer(0))
            ) {
                continue;
            }
            if deg == 0 {
                acc_terms.push(simp_c);
            } else {
                let pow_node = if deg == 1 {
                    target_sym
                } else {
                    graph.pow(target_sym, graph.integer(deg as i64))
                };
                if matches!(
                    graph.get(simp_c).kind,
                    ExprKind::Number(algebra_core::Number::Integer(1))
                ) {
                    acc_terms.push(pow_node);
                } else {
                    acc_terms.push(graph.mul([simp_c, pow_node]));
                }
            }
        }
        if acc_terms.is_empty() {
            graph.integer(0)
        } else {
            acc_terms
                .into_iter()
                .reduce(|a, b| graph.add([a, b]))
                .unwrap_or_else(|| graph.integer(0))
        }
    }

    /// Combine N-term sums and nested rational expressions into a single simplified fraction.
    pub fn together(graph: &ExprGraph, expr: ExprId) -> ExprId {
        tracing::debug!(expr = ?expr, "Performing rational fraction combination (together)");
        let (num, den) = to_single_fraction(graph, expr);
        let exp_num = AlgebraicTransformations::expand(graph, num);
        let exp_den = AlgebraicTransformations::expand(graph, den);
        if is_expr_one(graph, exp_den) {
            exp_num
        } else {
            cancel_num_den(graph, exp_num, exp_den)
        }
    }

    /// Cancel common polynomial and symbolic factors in fraction numerator and denominator.
    pub fn cancel(graph: &ExprGraph, expr: ExprId) -> ExprId {
        tracing::debug!(expr = ?expr, "Performing fraction factor cancellation");
        match &graph.get(expr).kind {
            ExprKind::Div(num, den) => cancel_num_den(graph, *num, *den),
            ExprKind::Add(_) | ExprKind::Sub(_, _) => {
                let (num, den) = to_single_fraction(graph, expr);
                cancel_num_den(graph, num, den)
            }
            ExprKind::Mul(_) | ExprKind::Pow(_, _) => {
                let (num, den) = to_single_fraction(graph, expr);
                if is_expr_one(graph, den) {
                    expr
                } else {
                    cancel_num_den(graph, num, den)
                }
            }
            _ => expr,
        }
    }

    /// Convert arbitrary-degree polynomial (or parametric polynomial) into Horner's nested multiplication form.
    pub fn horner(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> ExprId {
        use num_traits::{One as _, Signed as _, Zero as _};
        tracing::debug!(expr = ?expr, target = ?target, "Converting polynomial to Horner form");

        let target_sym = graph.symbol(&graph.symbols.resolve(target).unwrap_or_default());
        let Some(poly) = mv_poly_from_expr(graph, expr) else {
            return expr;
        };
        if poly.is_empty() {
            return graph.integer(0);
        }

        let mut max_deg = 0u32;
        let mut slices: std::collections::BTreeMap<u32, MvPoly> = std::collections::BTreeMap::new();
        for (mono, coeff) in poly {
            let mut deg = 0u32;
            let mut rest = Vec::new();
            for (atom, e) in mono {
                if atom == target_sym {
                    deg += e;
                } else {
                    rest.push((atom, e));
                }
            }
            max_deg = max_deg.max(deg);
            let sub = slices.entry(deg).or_default();
            let entry = sub
                .entry(rest.clone())
                .or_insert_with(num_rational::BigRational::zero);
            *entry += coeff;
            if entry.is_zero() {
                sub.remove(&rest);
            }
        }

        if max_deg == 0 {
            return mv_poly_to_expr(graph, slices.get(&0).unwrap_or(&MvPoly::new()));
        }

        let is_unit_coeff =
            |p: &MvPoly| -> bool { p.len() == 1 && p.get(&Vec::new()).is_some_and(|c| c.is_one()) };
        let is_neg_unit_coeff = |p: &MvPoly| -> bool {
            p.len() == 1
                && p.get(&Vec::new())
                    .is_some_and(|c| *c == -num_rational::BigRational::one())
        };

        let top = slices.get(&max_deg).cloned().unwrap_or_default();
        let mut acc = mv_poly_to_expr(graph, &top);
        let mut acc_is_one = is_unit_coeff(&top);
        let mut acc_is_neg_one = is_neg_unit_coeff(&top);

        for deg in (0..max_deg).rev() {
            acc = if acc_is_one {
                target_sym
            } else if acc_is_neg_one {
                graph.neg(target_sym)
            } else {
                graph.mul([acc, target_sym])
            };
            acc_is_one = false;
            acc_is_neg_one = false;

            if let Some(sub) = slices.get(&deg)
                && !sub.is_empty()
            {
                let is_const_neg = sub.len() == 1
                    && sub
                        .iter()
                        .next()
                        .is_some_and(|(m, c)| m.is_empty() && c.is_negative());
                if is_const_neg {
                    let neg_sub: MvPoly = sub.iter().map(|(k, v)| (k.clone(), -v)).collect();
                    let rhs = mv_poly_to_expr(graph, &neg_sub);
                    acc = graph.sub(acc, rhs);
                } else {
                    let rhs = mv_poly_to_expr(graph, sub);
                    acc = graph.add([acc, rhs]);
                }
            }
        }
        acc
    }

    /// Factor univariate and multivariate polynomials into irreducible factors over Q.
    pub fn factor(graph: &ExprGraph, expr: ExprId) -> ExprId {
        tracing::debug!(expr = ?expr, "Factoring polynomial expression");
        let mut sym_set = std::collections::BTreeSet::new();
        crate::poly::collect_expr_symbols(graph, expr, &mut sym_set);
        if sym_set.is_empty() {
            return expr;
        }
        let syms: Vec<SymbolId> = sym_set.into_iter().collect();
        if syms.len() == 1 {
            return factor_univariate_over_q(graph, expr, syms[0]).unwrap_or(expr);
        }
        factor_multivariate_over_q(graph, expr, &syms).unwrap_or(expr)
    }

    /// Exact partial fraction decomposition of rational functions $P(x)/Q(x)$ over $\mathbb{Q}\[x\]$.
    /// Handles improper fractions (polynomial long division), arbitrary degree, repeated roots $(x-r)^k$,
    /// and irreducible quadratic/higher factors using exact `BigRational` linear algebra.
    pub fn partial_fractions(graph: &ExprGraph, expr: ExprId, target: SymbolId) -> ExprId {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        use num_traits::{One as _, Signed as _, Zero as _};
        tracing::debug!(expr = ?expr, target = ?target, "Performing partial fractions decomposition");

        let target_sym = graph.symbol(&graph.symbols.resolve(target).unwrap_or_default());
        let (num_id, den_id) = match &graph.get(expr).kind {
            ExprKind::Div(n, d) => (*n, *d),
            ExprKind::Pow(b, e) => {
                if let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*e).kind
                    && k < 0
                {
                    let pk = -k;
                    let d = if pk == 1 {
                        *b
                    } else {
                        graph.pow(*b, graph.integer(pk))
                    };
                    (graph.integer(1), d)
                } else {
                    return expr;
                }
            }
            _ => return expr,
        };

        let Ok(p_num) = crate::poly::UniPoly::from_expr(graph, num_id, target) else {
            return expr;
        };
        let Ok(p_den) = crate::poly::UniPoly::from_expr(graph, den_id, target) else {
            return expr;
        };
        if p_den.is_zero() {
            return expr;
        }
        if p_den.degree() == 0 {
            let c = p_den.coeffs[0].clone();
            let scaled = crate::poly::UniPoly {
                coeffs: p_num.coeffs.iter().map(|q| q / &c).collect(),
            };
            return q_poly_to_expr(graph, &scaled.coeffs, target_sym);
        }

        // 1. Long division if improper
        let Ok((quot, rem_raw)) = p_num.div_rem(&p_den) else {
            return expr;
        };
        if rem_raw.is_zero() {
            return q_poly_to_expr(graph, &quot.coeffs, target_sym);
        }

        // 2. Reduce rem / p_den by polynomial GCD
        let g = rem_raw
            .gcd(&p_den)
            .unwrap_or_else(|_| crate::poly::UniPoly::constant(BigRational::one()));
        let rem_red = rem_raw.div_rem(&g).map(|(q, _)| q).unwrap_or(rem_raw);
        let den_red = p_den.div_rem(&g).map(|(q, _)| q).unwrap_or(p_den);
        if den_red.degree() == 0 {
            let c = den_red.coeffs[0].clone();
            let rem_scaled = crate::poly::UniPoly {
                coeffs: rem_red.coeffs.iter().map(|q| q / &c).collect(),
            };
            let full = crate::poly::UniPoly {
                coeffs: {
                    let len = quot.coeffs.len().max(rem_scaled.coeffs.len());
                    (0..len)
                        .map(|i| {
                            quot.coeffs
                                .get(i)
                                .cloned()
                                .unwrap_or_else(BigRational::zero)
                                + rem_scaled
                                    .coeffs
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(BigRational::zero)
                        })
                        .collect()
                },
            };
            return q_poly_to_expr(graph, &full.coeffs, target_sym);
        }

        // 3. Extract leading scale from den_red and factor primitive integer form
        let lc_den = den_red
            .coeffs
            .last()
            .cloned()
            .unwrap_or_else(BigRational::one);
        let den_monic = crate::poly::UniPoly {
            coeffs: den_red.coeffs.iter().map(|c| c / &lc_den).collect(),
        };
        let rem_scaled: Vec<BigRational> = rem_red.coeffs.iter().map(|c| c / &lc_den).collect();

        // Factor den_monic into irreducible primitive Z[x] components with multiplicity
        let mut irred_factors: Vec<(Vec<BigInt>, u32)> = Vec::new();
        for (sq_f, mult) in square_free_decomposition(&den_monic.coeffs) {
            let prim = make_primitive_int_poly(&sq_f);
            for irr in factor_sqfree_primitive_int_poly(&prim) {
                if let Some(pos) = irred_factors.iter().position(|(ef, _)| ef == &irr) {
                    irred_factors[pos].1 += mult;
                } else {
                    irred_factors.push((irr, mult));
                }
            }
        }
        irred_factors.sort_by(|(fa, _), (fb, _)| {
            fa.len()
                .cmp(&fb.len())
                .then_with(|| fa.iter().rev().cmp(fb.iter().rev()))
        });

        let deg_q = den_monic.degree();
        // If irreducible single factor of multiplicity 1 (e.g. 1/(x^2+1)), cannot decompose further
        if irred_factors.len() == 1 && irred_factors[0].1 == 1 && quot.is_zero() {
            return expr;
        }

        // 4. Construct exact Q[x] linear system for undetermined coefficients
        // Basis entry metadata: (factor_idx, power_k, deg_j)
        let mut basis_meta: Vec<(usize, u32, usize)> = Vec::with_capacity(deg_q);
        let mut basis_polys: Vec<Vec<BigRational>> = Vec::with_capacity(deg_q);

        for (fi_idx, (f_int, mult)) in irred_factors.iter().enumerate() {
            let f_q = crate::poly::UniPoly {
                coeffs: f_int
                    .iter()
                    .cloned()
                    .map(BigRational::from_integer)
                    .collect(),
            };
            let d_i = f_q.degree();
            let mut f_pow = f_q.clone();
            for k in 1..=*mult {
                let cofactor = den_monic.div_rem(&f_pow).map(|(q, _)| q).ok();
                let Some(cof) = cofactor else { return expr };
                for j in 0..d_i {
                    // x^j * cof
                    let mut col = vec![BigRational::zero(); deg_q];
                    for (idx, c) in cof.coeffs.iter().enumerate() {
                        if idx + j < deg_q {
                            col[idx + j] = c.clone();
                        }
                    }
                    basis_meta.push((fi_idx, k, j));
                    basis_polys.push(col);
                }
                if k < *mult {
                    f_pow = poly_q_mul(&f_pow, &f_q);
                }
            }
        }

        if basis_polys.len() != deg_q {
            return expr;
        }

        let mut matrix = vec![vec![BigRational::zero(); deg_q]; deg_q];
        for (col_idx, col_vec) in basis_polys.iter().enumerate() {
            for (row_idx, val) in col_vec.iter().enumerate() {
                matrix[row_idx][col_idx] = val.clone();
            }
        }
        let mut rhs = vec![BigRational::zero(); deg_q];
        for (i, c) in rem_scaled.iter().enumerate() {
            if i < deg_q {
                rhs[i] = c.clone();
            }
        }

        let Some(sol) = rational_gaussian_solve(&mut matrix, &mut rhs) else {
            return expr;
        };

        // 5. Reassemble polynomial quotient + non-zero partial fraction terms
        let mut terms_pos_neg: Vec<(bool, ExprId)> = Vec::new();
        if !quot.is_zero() {
            terms_pos_neg.push((false, q_poly_to_expr(graph, &quot.coeffs, target_sym)));
        }

        // Group solved coefficients per (fi_idx, k)
        let mut cursor = 0usize;
        for (fi_idx, (f_int, mult)) in irred_factors.iter().enumerate() {
            let d_i = f_int.len() - 1;
            let f_expr = int_poly_to_expr(graph, f_int, target_sym);
            for k in 1..=*mult {
                let mut num_coeffs = Vec::with_capacity(d_i);
                for j in 0..d_i {
                    let _ = (fi_idx, k, j);
                    num_coeffs.push(sol[cursor].clone());
                    cursor += 1;
                }
                while num_coeffs.len() > 1 && num_coeffs.last().is_some_and(|c| c.is_zero()) {
                    num_coeffs.pop();
                }
                if num_coeffs.iter().all(|c| c.is_zero()) {
                    continue;
                }
                let den_part = if k == 1 {
                    f_expr
                } else {
                    graph.pow(f_expr, graph.integer(k as i64))
                };
                if num_coeffs.len() == 1 && num_coeffs[0].is_negative() {
                    let abs_num = crate::poly::expr_from_rational(graph, &(-&num_coeffs[0]));
                    terms_pos_neg.push((true, graph.div(abs_num, den_part)));
                } else {
                    let num_part = q_poly_to_expr(graph, &num_coeffs, target_sym);
                    terms_pos_neg.push((false, graph.div(num_part, den_part)));
                }
            }
        }

        if terms_pos_neg.is_empty() {
            return graph.integer(0);
        }
        let (neg0, e0) = terms_pos_neg[0];
        let mut acc = if neg0 { graph.neg(e0) } else { e0 };
        for &(is_neg, t) in &terms_pos_neg[1..] {
            acc = if is_neg {
                graph.sub(acc, t)
            } else {
                graph.add([acc, t])
            };
        }
        acc
    }
}

fn is_expr_one(graph: &ExprGraph, id: ExprId) -> bool {
    match &graph.get(id).kind {
        ExprKind::Number(algebra_core::Number::Integer(1)) => true,
        ExprKind::Number(algebra_core::Number::BigInteger(b)) => b == &num_bigint::BigInt::from(1),
        _ => false,
    }
}

fn flatten_signed_terms(graph: &ExprGraph, id: ExprId, neg: bool, out: &mut Vec<(ExprId, bool)>) {
    match &graph.get(id).kind {
        ExprKind::Add(terms) => {
            for &t in terms {
                flatten_signed_terms(graph, t, neg, out);
            }
        }
        ExprKind::Sub(l, r) => {
            flatten_signed_terms(graph, *l, neg, out);
            flatten_signed_terms(graph, *r, !neg, out);
        }
        ExprKind::Neg(inner) => flatten_signed_terms(graph, *inner, !neg, out),
        _ => out.push((id, neg)),
    }
}

fn extract_target_power(graph: &ExprGraph, term: ExprId, target_sym: ExprId) -> (u32, ExprId) {
    if term == target_sym {
        return (1, graph.integer(1));
    }
    match &graph.get(term).kind {
        ExprKind::Pow(b, e) if *b == target_sym => {
            if let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*e).kind
                && k >= 0
            {
                return (k as u32, graph.integer(1));
            }
            (0, term)
        }
        ExprKind::Mul(factors) => {
            let mut deg = 0u32;
            let mut rest = Vec::new();
            for &f in factors {
                if f == target_sym {
                    deg += 1;
                } else if let ExprKind::Pow(b, e) = &graph.get(f).kind
                    && *b == target_sym
                    && let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*e).kind
                    && k >= 0
                {
                    deg += k as u32;
                } else {
                    rest.push(f);
                }
            }
            let c = if rest.is_empty() {
                graph.integer(1)
            } else if rest.len() == 1 {
                rest[0]
            } else {
                graph.mul(rest)
            };
            (deg, c)
        }
        _ => (0, term),
    }
}

fn to_single_fraction(graph: &ExprGraph, expr: ExprId) -> (ExprId, ExprId) {
    match &graph.get(expr).kind {
        ExprKind::Div(a, b) => {
            let (na, da) = to_single_fraction(graph, *a);
            let (nb, db) = to_single_fraction(graph, *b);
            let num = if is_expr_one(graph, db) {
                na
            } else {
                graph.mul([na, db])
            };
            let den = if is_expr_one(graph, da) {
                nb
            } else if is_expr_one(graph, nb) {
                da
            } else {
                graph.mul([da, nb])
            };
            (num, den)
        }
        ExprKind::Neg(inner) => {
            let (n, d) = to_single_fraction(graph, *inner);
            (graph.neg(n), d)
        }
        ExprKind::Add(_) | ExprKind::Sub(_, _) => {
            let mut signed = Vec::new();
            flatten_signed_terms(graph, expr, false, &mut signed);
            let mut fracs: Vec<(ExprId, ExprId)> = Vec::with_capacity(signed.len());
            for (term, is_neg) in signed {
                let (n, d) = to_single_fraction(graph, term);
                fracs.push((if is_neg { graph.neg(n) } else { n }, d));
            }
            let mut unique_dens: Vec<ExprId> = Vec::new();
            for &(_, d) in &fracs {
                if !is_expr_one(graph, d) && !unique_dens.contains(&d) {
                    unique_dens.push(d);
                }
            }
            if unique_dens.is_empty() {
                let ns: Vec<ExprId> = fracs.into_iter().map(|(n, _)| n).collect();
                let num = ns
                    .into_iter()
                    .reduce(|a, b| graph.add([a, b]))
                    .unwrap_or_else(|| graph.integer(0));
                return (num, graph.integer(1));
            }
            let common_den = if unique_dens.len() == 1 {
                unique_dens[0]
            } else {
                graph.mul(unique_dens.clone())
            };
            let mut num_terms = Vec::with_capacity(fracs.len());
            for (n, d) in fracs {
                let mut mul_parts = vec![n];
                for &ud in &unique_dens {
                    if ud != d {
                        mul_parts.push(ud);
                    }
                }
                num_terms.push(if mul_parts.len() == 1 {
                    mul_parts[0]
                } else {
                    graph.mul(mul_parts)
                });
            }
            let num = num_terms
                .into_iter()
                .reduce(|a, b| graph.add([a, b]))
                .unwrap_or_else(|| graph.integer(0));
            (num, common_den)
        }
        ExprKind::Mul(factors) => {
            let mut nums = Vec::new();
            let mut dens = Vec::new();
            for &f in factors {
                let (nf, df) = to_single_fraction(graph, f);
                if !is_expr_one(graph, nf) {
                    nums.push(nf);
                }
                if !is_expr_one(graph, df) {
                    dens.push(df);
                }
            }
            let n = if nums.is_empty() {
                graph.integer(1)
            } else if nums.len() == 1 {
                nums[0]
            } else {
                graph.mul(nums)
            };
            let d = if dens.is_empty() {
                graph.integer(1)
            } else if dens.len() == 1 {
                dens[0]
            } else {
                graph.mul(dens)
            };
            (n, d)
        }
        ExprKind::Pow(base, exp) => {
            if let ExprKind::Number(algebra_core::Number::Integer(k)) = graph.get(*exp).kind
                && (-16..=16).contains(&k)
                && k != 0
            {
                let (nb, db) = to_single_fraction(graph, *base);
                let ak = k.unsigned_abs() as i64;
                let np = if ak == 1 {
                    nb
                } else {
                    graph.pow(nb, graph.integer(ak))
                };
                let dp = if ak == 1 {
                    db
                } else if is_expr_one(graph, db) {
                    graph.integer(1)
                } else {
                    graph.pow(db, graph.integer(ak))
                };
                return if k < 0 { (dp, np) } else { (np, dp) };
            }
            (expr, graph.integer(1))
        }
        _ => (expr, graph.integer(1)),
    }
}

fn cancel_num_den(graph: &ExprGraph, num: ExprId, den: ExprId) -> ExprId {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    if num == den {
        return graph.integer(1);
    }
    if let (Some(mut pn), Some(mut pd)) =
        (mv_poly_from_expr(graph, num), mv_poly_from_expr(graph, den))
    {
        if pd.is_empty() {
            return graph.div(num, den);
        }
        if pn.is_empty() {
            return graph.integer(0);
        }

        // Compute exact multivariate polynomial GCD over shared atoms
        let g = mv_poly_gcd(&pn, &pd);
        if !g.is_empty() && !(g.len() == 1 && g.contains_key(&Vec::new())) {
            if let (Some(qn), Some(qd)) = (mv_poly_div_exact(&pn, &g), mv_poly_div_exact(&pd, &g)) {
                pn = qn;
                pd = qd;
            }
        }

        // Normalize rational content between pn and pd (clear combined denom lcm, divide by gcd of integers)
        let mut den_lcm = BigInt::one();
        for c in pn.values().chain(pd.values()) {
            den_lcm = den_lcm.lcm(c.denom());
        }
        let scale = BigRational::from_integer(den_lcm);
        let mut int_gcd = BigInt::zero();
        for c in pn.values().chain(pd.values()) {
            int_gcd = int_gcd.gcd(&(c * &scale).to_integer().abs());
        }
        if !int_gcd.is_zero() {
            let norm = &scale / BigRational::from_integer(int_gcd);
            for v in pn.values_mut() {
                *v *= &norm;
            }
            for v in pd.values_mut() {
                *v *= &norm;
            }
        }

        // Keep denominator leading monomial coefficient positive
        if let Some((_, lead_c)) = mv_leading_term(&pd)
            && lead_c.is_negative()
        {
            for v in pn.values_mut() {
                *v = -v.clone();
            }
            for v in pd.values_mut() {
                *v = -v.clone();
            }
        }

        if pd.len() == 1
            && let Some(c) = pd.get(&Vec::new()).cloned()
        {
            let scaled: MvPoly = pn.into_iter().map(|(k, v)| (k, v / &c)).collect();
            return mv_poly_to_expr(graph, &scaled);
        }
        return graph.div(mv_poly_to_expr(graph, &pn), mv_poly_to_expr(graph, &pd));
    }

    let simp_num = crate::Simplifier::simplify(graph, num).unwrap_or(num);
    let simp_den = crate::Simplifier::simplify(graph, den).unwrap_or(den);
    if simp_num == simp_den {
        graph.integer(1)
    } else {
        graph.div(simp_num, simp_den)
    }
}

fn q_poly_to_expr(
    graph: &ExprGraph,
    coeffs: &[num_rational::BigRational],
    var_sym: ExprId,
) -> ExprId {
    use num_traits::{One as _, Signed as _, Zero as _};
    let mut terms: Vec<(usize, num_rational::BigRational)> = Vec::new();
    for (deg, c) in coeffs.iter().enumerate().rev() {
        if !c.is_zero() {
            terms.push((deg, c.clone()));
        }
    }
    if terms.is_empty() {
        return graph.integer(0);
    }
    let build_abs = |deg: usize, c_abs: &num_rational::BigRational| -> ExprId {
        let c_expr = crate::poly::expr_from_rational(graph, c_abs);
        if deg == 0 {
            c_expr
        } else {
            let v_pow = if deg == 1 {
                var_sym
            } else {
                graph.pow(var_sym, graph.integer(deg as i64))
            };
            if c_abs.is_one() {
                v_pow
            } else {
                graph.mul([c_expr, v_pow])
            }
        }
    };
    let (d0, ref c0) = terms[0];
    let mut acc = if c0.is_negative() {
        graph.neg(build_abs(d0, &(-c0)))
    } else {
        build_abs(d0, c0)
    };
    for (d, c) in &terms[1..] {
        if c.is_negative() {
            acc = graph.sub(acc, build_abs(*d, &(-c)));
        } else {
            acc = graph.add([acc, build_abs(*d, c)]);
        }
    }
    acc
}

fn poly_q_mul(a: &crate::poly::UniPoly, b: &crate::poly::UniPoly) -> crate::poly::UniPoly {
    use num_rational::BigRational;
    use num_traits::Zero as _;
    if a.is_zero() || b.is_zero() {
        return crate::poly::UniPoly::zero();
    }
    let mut out = vec![BigRational::zero(); a.coeffs.len() + b.coeffs.len() - 1];
    for (i, ca) in a.coeffs.iter().enumerate() {
        for (j, cb) in b.coeffs.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    while out.len() > 1 && out.last().is_some_and(|c| c.is_zero()) {
        out.pop();
    }
    crate::poly::UniPoly { coeffs: out }
}

fn rational_gaussian_solve(
    mat: &mut [Vec<num_rational::BigRational>],
    rhs: &mut [num_rational::BigRational],
) -> Option<Vec<num_rational::BigRational>> {
    use num_rational::BigRational;
    use num_traits::Zero as _;
    let n = rhs.len();
    for col in 0..n {
        let pivot = (col..n).find(|&r| !mat[r][col].is_zero())?;
        mat.swap(col, pivot);
        rhs.swap(col, pivot);
        let pv = mat[col][col].clone();
        for j in col..n {
            mat[col][j] /= &pv;
        }
        rhs[col] /= &pv;
        let row_col = mat[col].clone();
        let rhs_col = rhs[col].clone();
        for r in 0..n {
            if r == col || mat[r][col].is_zero() {
                continue;
            }
            let factor = mat[r][col].clone();
            for j in col..n {
                let sub = &factor * &row_col[j];
                mat[r][j] -= sub;
            }
            rhs[r] -= &factor * &rhs_col;
        }
    }
    let mut sol = Vec::with_capacity(n);
    for r in 0..n {
        sol.push(std::mem::replace(&mut rhs[r], BigRational::zero()));
    }
    Some(sol)
}

fn mv_lex_cmp(ma: &MonomialKey, mb: &MonomialKey) -> std::cmp::Ordering {
    let mut ia = 0;
    let mut ib = 0;
    while ia < ma.len() && ib < mb.len() {
        let (va, ea) = ma[ia];
        let (vb, eb) = mb[ib];
        if va == vb {
            if ea != eb {
                return ea.cmp(&eb);
            }
            ia += 1;
            ib += 1;
        } else if va < vb {
            return std::cmp::Ordering::Greater;
        } else {
            return std::cmp::Ordering::Less;
        }
    }
    if ia < ma.len() {
        std::cmp::Ordering::Greater
    } else if ib < mb.len() {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Equal
    }
}

fn mv_leading_term(p: &MvPoly) -> Option<(MonomialKey, num_rational::BigRational)> {
    p.iter()
        .max_by(|(ma, _), (mb, _)| {
            let da: u32 = ma.iter().map(|x| x.1).sum();
            let db: u32 = mb.iter().map(|x| x.1).sum();
            da.cmp(&db).then_with(|| mv_lex_cmp(ma, mb))
        })
        .map(|(m, c)| (m.clone(), c.clone()))
}

fn mv_div_monomial(a: &MonomialKey, b: &MonomialKey) -> Option<MonomialKey> {
    let mut out = Vec::new();
    for &(vb, eb) in b {
        let ea = a
            .iter()
            .find(|&&(va, _)| va == vb)
            .map(|&(_, e)| e)
            .unwrap_or(0);
        if ea < eb {
            return None;
        }
    }
    for &(va, ea) in a {
        let eb = b
            .iter()
            .find(|&&(vb, _)| vb == va)
            .map(|&(_, e)| e)
            .unwrap_or(0);
        if ea > eb {
            out.push((va, ea - eb));
        }
    }
    Some(out)
}

fn mv_poly_div_exact(a: &MvPoly, b: &MvPoly) -> Option<MvPoly> {
    use num_traits::Zero as _;
    let (lt_b, lc_b) = mv_leading_term(b)?;
    let mut rem = a.clone();
    let mut quot = MvPoly::new();
    let mut steps = 0usize;
    while !rem.is_empty() {
        steps += 1;
        if steps > 4096 {
            return None;
        }
        let (lt_r, lc_r) = mv_leading_term(&rem)?;
        let dm = mv_div_monomial(&lt_r, &lt_b)?;
        let dc = &lc_r / &lc_b;
        let mut q_term = MvPoly::new();
        q_term.insert(dm.clone(), dc.clone());
        let entry = quot
            .entry(dm)
            .or_insert_with(num_rational::BigRational::zero);
        *entry += &dc;
        let sub_term = mv_mul(&q_term, b);
        rem = mv_sub(&rem, &sub_term);
    }
    Some(quot)
}

fn mv_atoms(p: &MvPoly) -> std::collections::BTreeSet<ExprId> {
    let mut out = std::collections::BTreeSet::new();
    for m in p.keys() {
        for &(a, _) in m {
            out.insert(a);
        }
    }
    out
}

fn mv_to_univ_in_var(p: &MvPoly, var: ExprId) -> Vec<MvPoly> {
    use num_traits::Zero as _;
    let mut max_deg = 0usize;
    for m in p.keys() {
        if let Some(&(_, e)) = m.iter().find(|&&(a, _)| a == var) {
            max_deg = max_deg.max(e as usize);
        }
    }
    let mut out = vec![MvPoly::new(); max_deg + 1];
    for (m, c) in p {
        let mut deg = 0usize;
        let mut rest = Vec::new();
        for &(a, e) in m {
            if a == var {
                deg = e as usize;
            } else {
                rest.push((a, e));
            }
        }
        let e_ref = out[deg]
            .entry(rest.clone())
            .or_insert_with(num_rational::BigRational::zero);
        *e_ref += c;
        if e_ref.is_zero() {
            out[deg].remove(&rest);
        }
    }
    while out.len() > 1 && out.last().is_some_and(|c| c.is_empty()) {
        out.pop();
    }
    out
}

fn univ_to_mv(u: &[MvPoly], var: ExprId) -> MvPoly {
    use num_traits::Zero as _;
    let mut out = MvPoly::new();
    for (deg, cpoly) in u.iter().enumerate() {
        for (m, c) in cpoly {
            let mut nm = m.clone();
            if deg > 0 {
                nm.push((var, deg as u32));
                nm.sort_by_key(|&(a, _)| a);
            }
            let e_ref = out
                .entry(nm.clone())
                .or_insert_with(num_rational::BigRational::zero);
            *e_ref += c;
            if e_ref.is_zero() {
                out.remove(&nm);
            }
        }
    }
    out
}

fn mv_one() -> MvPoly {
    let mut m = MvPoly::new();
    m.insert(
        Vec::new(),
        num_rational::BigRational::from_integer(1.into()),
    );
    m
}

fn mv_poly_gcd(a: &MvPoly, b: &MvPoly) -> MvPoly {
    use num_traits::One as _;
    if a.is_empty() {
        return b.clone();
    }
    if b.is_empty() {
        return a.clone();
    }
    if mv_poly_div_exact(a, b).is_some() {
        return b.clone();
    }
    if mv_poly_div_exact(b, a).is_some() {
        return a.clone();
    }

    let atoms_a = mv_atoms(a);
    let atoms_b = mv_atoms(b);
    let Some(&var) = atoms_a.intersection(&atoms_b).next() else {
        return mv_one();
    };

    // Fast path for univariate case
    if atoms_a.len() == 1 && atoms_b.len() == 1 {
        let ua = crate::poly::UniPoly {
            coeffs: mv_to_univ_in_var(a, var)
                .into_iter()
                .map(|c| {
                    c.get(&Vec::new())
                        .cloned()
                        .unwrap_or_else(|| num_rational::BigRational::from_integer(0.into()))
                })
                .collect(),
        };
        let ub = crate::poly::UniPoly {
            coeffs: mv_to_univ_in_var(b, var)
                .into_iter()
                .map(|c| {
                    c.get(&Vec::new())
                        .cloned()
                        .unwrap_or_else(|| num_rational::BigRational::from_integer(0.into()))
                })
                .collect(),
        };
        if let Ok(ug) = ua.gcd(&ub) {
            let mut m = MvPoly::new();
            for (deg, c) in ug.coeffs.into_iter().enumerate() {
                if c != num_rational::BigRational::from_integer(0.into()) {
                    let key = if deg == 0 {
                        Vec::new()
                    } else {
                        vec![(var, deg as u32)]
                    };
                    m.insert(key, c);
                }
            }
            return if m.is_empty() { mv_one() } else { m };
        }
    }

    let content_and_pp = |u: Vec<MvPoly>| -> (MvPoly, Vec<MvPoly>) {
        let mut g = MvPoly::new();
        for c in &u {
            if !c.is_empty() {
                g = if g.is_empty() {
                    c.clone()
                } else {
                    mv_poly_gcd(&g, c)
                };
            }
        }
        if g.is_empty() || (g.len() == 1 && g.contains_key(&Vec::new())) {
            return (mv_one(), u);
        }
        let pp: Option<Vec<MvPoly>> = u
            .iter()
            .map(|c| {
                if c.is_empty() {
                    Some(MvPoly::new())
                } else {
                    mv_poly_div_exact(c, &g)
                }
            })
            .collect();
        match pp {
            Some(p) => (g, p),
            None => (mv_one(), u),
        }
    };

    let (ca, mut u) = content_and_pp(mv_to_univ_in_var(a, var));
    let (cb, mut v) = content_and_pp(mv_to_univ_in_var(b, var));
    let content_g = mv_poly_gcd(&ca, &cb);

    if u.len() < v.len() {
        std::mem::swap(&mut u, &mut v);
    }

    for _ in 0..32 {
        let deg_v = v.len().saturating_sub(1);
        if deg_v == 0 {
            return if content_g.is_empty() {
                mv_one()
            } else {
                content_g
            };
        }
        // Pseudo-remainder of u mod v in `var`
        let lc_v = v[deg_v].clone();
        let mut r = u.clone();
        let mut rem_steps = 0usize;
        while r.len() >= v.len() && !r.is_empty() {
            rem_steps += 1;
            if rem_steps > 64 {
                return content_g;
            }
            let deg_r = r.len() - 1;
            let shift = deg_r - deg_v;
            let lc_r = r[deg_r].clone();
            let mut next_r = vec![MvPoly::new(); r.len()];
            for i in 0..deg_r {
                next_r[i] = mv_mul(&r[i], &lc_v);
            }
            for i in 0..deg_v {
                let sub_t = mv_mul(&v[i], &lc_r);
                next_r[i + shift] = mv_sub(&next_r[i + shift], &sub_t);
            }
            while !next_r.is_empty() && next_r.last().is_some_and(|c| c.is_empty()) {
                next_r.pop();
            }
            r = next_r;
        }
        if r.is_empty() {
            let mut res = univ_to_mv(&v, var);
            res = mv_mul(&res, &content_g);
            // Strip scalar content if unit
            if res.len() == 1
                && res
                    .iter()
                    .next()
                    .is_some_and(|(m, c)| m.is_empty() && !c.is_one())
            {
                return mv_one();
            }
            return res;
        }
        u = v;
        v = content_and_pp(r).1;
    }

    content_g
}

type MonomialKey = Vec<(ExprId, u32)>;
type MvPoly = std::collections::BTreeMap<MonomialKey, num_rational::BigRational>;

fn rat_from_expr(graph: &ExprGraph, id: ExprId) -> Option<num_rational::BigRational> {
    use num_traits::Zero as _;
    match &graph.get(id).kind {
        ExprKind::Number(n) => crate::poly::rational_from_number(n).ok(),
        ExprKind::Neg(inner) => Some(-rat_from_expr(graph, *inner)?),
        ExprKind::Div(a, b) => {
            let qa = rat_from_expr(graph, *a)?;
            let qb = rat_from_expr(graph, *b)?;
            if qb.is_zero() { None } else { Some(qa / qb) }
        }
        _ => None,
    }
}

fn mul_monomial(a: &MonomialKey, b: &MonomialKey) -> MonomialKey {
    let mut map: std::collections::BTreeMap<ExprId, u32> = std::collections::BTreeMap::new();
    for &(v, e) in a {
        *map.entry(v).or_insert(0) += e;
    }
    for &(v, e) in b {
        *map.entry(v).or_insert(0) += e;
    }
    map.into_iter().filter(|&(_, e)| e > 0).collect()
}

fn mv_add(a: &MvPoly, b: &MvPoly) -> MvPoly {
    use num_traits::Zero as _;
    let mut out = a.clone();
    for (m, c) in b {
        let entry = out
            .entry(m.clone())
            .or_insert_with(num_rational::BigRational::zero);
        *entry += c;
        if entry.is_zero() {
            out.remove(m);
        }
    }
    out
}

fn mv_sub(a: &MvPoly, b: &MvPoly) -> MvPoly {
    use num_traits::Zero as _;
    let mut out = a.clone();
    for (m, c) in b {
        let entry = out
            .entry(m.clone())
            .or_insert_with(num_rational::BigRational::zero);
        *entry -= c;
        if entry.is_zero() {
            out.remove(m);
        }
    }
    out
}

fn mv_mul(a: &MvPoly, b: &MvPoly) -> MvPoly {
    use num_traits::Zero as _;
    let mut out: MvPoly = std::collections::BTreeMap::new();
    for (ma, ca) in a {
        for (mb, cb) in b {
            let m = mul_monomial(ma, mb);
            let prod = ca * cb;
            let entry = out
                .entry(m.clone())
                .or_insert_with(num_rational::BigRational::zero);
            *entry += prod;
            if entry.is_zero() {
                out.remove(&m);
            }
        }
    }
    out
}

fn mv_pow(base: &MvPoly, mut exp: u32) -> MvPoly {
    let mut acc: MvPoly = std::collections::BTreeMap::new();
    acc.insert(
        Vec::new(),
        num_rational::BigRational::from_integer(1.into()),
    );
    let mut cur = base.clone();
    while exp > 0 {
        if exp & 1 == 1 {
            acc = mv_mul(&acc, &cur);
        }
        exp >>= 1;
        if exp > 0 {
            cur = mv_mul(&cur, &cur);
        }
    }
    acc
}

fn mv_poly_from_expr(graph: &ExprGraph, expr: ExprId) -> Option<MvPoly> {
    use num_traits::{Signed as _, ToPrimitive as _, Zero as _};
    match &graph.get(expr).kind {
        ExprKind::Number(n) => {
            let q = crate::poly::rational_from_number(n).ok()?;
            let mut m = MvPoly::new();
            if !q.is_zero() {
                m.insert(Vec::new(), q);
            }
            Some(m)
        }
        ExprKind::Symbol(_) => {
            let mut m = MvPoly::new();
            m.insert(
                vec![(expr, 1)],
                num_rational::BigRational::from_integer(1.into()),
            );
            Some(m)
        }
        ExprKind::Function { .. } => {
            let mut m = MvPoly::new();
            m.insert(
                vec![(expr, 1)],
                num_rational::BigRational::from_integer(1.into()),
            );
            Some(m)
        }
        ExprKind::Neg(inner) => {
            let p = mv_poly_from_expr(graph, *inner)?;
            Some(p.into_iter().map(|(k, v)| (k, -v)).collect())
        }
        ExprKind::Add(terms) => {
            let mut acc = MvPoly::new();
            for &t in terms {
                let pt = mv_poly_from_expr(graph, t)?;
                acc = mv_add(&acc, &pt);
            }
            Some(acc)
        }
        ExprKind::Sub(a, b) => {
            let pa = mv_poly_from_expr(graph, *a)?;
            let pb = mv_poly_from_expr(graph, *b)?;
            Some(mv_sub(&pa, &pb))
        }
        ExprKind::Mul(factors) => {
            let mut acc = MvPoly::new();
            acc.insert(
                Vec::new(),
                num_rational::BigRational::from_integer(1.into()),
            );
            for &f in factors {
                let pf = mv_poly_from_expr(graph, f)?;
                acc = mv_mul(&acc, &pf);
            }
            Some(acc)
        }
        ExprKind::Div(num, den) => {
            let pn = mv_poly_from_expr(graph, *num)?;
            let qd = rat_from_expr(graph, *den)?;
            if qd.is_zero() {
                return None;
            }
            Some(pn.into_iter().map(|(k, v)| (k, v / &qd)).collect())
        }
        ExprKind::Pow(base, exp) => {
            if let Some(qe) = rat_from_expr(graph, *exp)
                && qe.is_integer()
                && !qe.is_negative()
                && let Some(n) = qe.numer().to_u32()
                && n <= 32
            {
                let pb = mv_poly_from_expr(graph, *base)?;
                return Some(mv_pow(&pb, n));
            }
            let mut m = MvPoly::new();
            m.insert(
                vec![(expr, 1)],
                num_rational::BigRational::from_integer(1.into()),
            );
            Some(m)
        }
        _ => None,
    }
}

fn atom_sort_name(graph: &ExprGraph, id: ExprId) -> String {
    match &graph.get(id).kind {
        ExprKind::Symbol(s) => graph.symbols.resolve(*s).unwrap_or_default(),
        _ => format!("~{}", id.0),
    }
}

fn mv_poly_to_expr(graph: &ExprGraph, poly: &MvPoly) -> ExprId {
    use num_traits::{One as _, Signed as _};
    if poly.is_empty() {
        return graph.integer(0);
    }

    let mut entries: Vec<(MonomialKey, num_rational::BigRational)> = poly
        .iter()
        .map(|(m, c)| {
            let mut sm = m.clone();
            sm.sort_by_key(|a| atom_sort_name(graph, a.0));
            (sm, c.clone())
        })
        .collect();

    entries.sort_by(|(ma, _), (mb, _)| {
        let da: u32 = ma.iter().map(|x| x.1).sum();
        let db: u32 = mb.iter().map(|x| x.1).sum();
        db.cmp(&da).then_with(|| {
            let sa: Vec<_> = ma
                .iter()
                .map(|&(v, e)| (atom_sort_name(graph, v), std::cmp::Reverse(e)))
                .collect();
            let sb: Vec<_> = mb
                .iter()
                .map(|&(v, e)| (atom_sort_name(graph, v), std::cmp::Reverse(e)))
                .collect();
            sa.cmp(&sb)
        })
    });

    let build_abs_term = |m: &MonomialKey, c_abs: &num_rational::BigRational| -> ExprId {
        let mut factors = Vec::new();
        if !c_abs.is_one() || m.is_empty() {
            factors.push(crate::poly::expr_from_rational(graph, c_abs));
        }
        for &(var, exp) in m {
            if exp == 1 {
                factors.push(var);
            } else {
                factors.push(graph.pow(var, graph.integer(exp as i64)));
            }
        }
        if factors.is_empty() {
            graph.integer(1)
        } else if factors.len() == 1 {
            factors[0]
        } else {
            graph.mul(factors)
        }
    };

    let (m0, c0) = &entries[0];
    let mut acc = if c0.is_negative() {
        graph.neg(build_abs_term(m0, &(-c0)))
    } else {
        build_abs_term(m0, c0)
    };

    for (m, c) in &entries[1..] {
        if c.is_negative() {
            let t = build_abs_term(m, &(-c));
            acc = graph.sub(acc, t);
        } else {
            let t = build_abs_term(m, c);
            acc = graph.add([acc, t]);
        }
    }
    acc
}

fn expand_structural(graph: &ExprGraph, expr: ExprId) -> ExprId {
    let node = graph.get(expr);
    match &node.kind {
        ExprKind::Neg(inner) => graph.neg(AlgebraicTransformations::expand(graph, *inner)),
        ExprKind::Add(terms) => {
            let ts: Vec<ExprId> = terms
                .iter()
                .map(|&t| AlgebraicTransformations::expand(graph, t))
                .collect();
            graph.add(ts)
        }
        ExprKind::Sub(l, r) => {
            let el = AlgebraicTransformations::expand(graph, *l);
            let er = AlgebraicTransformations::expand(graph, *r);
            graph.sub(el, er)
        }
        ExprKind::Div(n, d) => {
            let en = AlgebraicTransformations::expand(graph, *n);
            let ed = AlgebraicTransformations::expand(graph, *d);
            graph.div(en, ed)
        }
        ExprKind::Mul(factors) => {
            let mut acc = AlgebraicTransformations::expand(graph, factors[0]);
            for &f in &factors[1..] {
                let ef = AlgebraicTransformations::expand(graph, f);
                acc = distribute_binary_mul(graph, acc, ef);
            }
            acc
        }
        ExprKind::Pow(base, exp) => {
            let eb = AlgebraicTransformations::expand(graph, *base);
            if let ExprKind::Number(algebra_core::Number::Integer(p)) = graph.get(*exp).kind
                && (0..=16).contains(&p)
            {
                if p == 0 {
                    return graph.integer(1);
                }
                let mut acc = eb;
                for _ in 1..p {
                    acc = distribute_binary_mul(graph, acc, eb);
                }
                return acc;
            }
            graph.pow(eb, *exp)
        }
        _ => expr,
    }
}

fn distribute_binary_mul(graph: &ExprGraph, a: ExprId, b: ExprId) -> ExprId {
    match (&graph.get(a).kind, &graph.get(b).kind) {
        (ExprKind::Add(ta), ExprKind::Add(tb)) => {
            let mut out = Vec::new();
            for &x in ta {
                for &y in tb {
                    out.push(distribute_binary_mul(graph, x, y));
                }
            }
            graph.add(out)
        }
        (ExprKind::Add(ta), _) => {
            let out: Vec<ExprId> = ta
                .iter()
                .map(|&x| distribute_binary_mul(graph, x, b))
                .collect();
            graph.add(out)
        }
        (_, ExprKind::Add(tb)) => {
            let out: Vec<ExprId> = tb
                .iter()
                .map(|&y| distribute_binary_mul(graph, a, y))
                .collect();
            graph.add(out)
        }
        (ExprKind::Sub(a1, a2), _) => graph.sub(
            distribute_binary_mul(graph, *a1, b),
            distribute_binary_mul(graph, *a2, b),
        ),
        (_, ExprKind::Sub(b1, b2)) => graph.sub(
            distribute_binary_mul(graph, a, *b1),
            distribute_binary_mul(graph, a, *b2),
        ),
        _ => graph.mul([a, b]),
    }
}

// =========================================================================
// Univariate & Multivariate Polynomial Factorization over Q
// =========================================================================

fn factor_univariate_over_q(graph: &ExprGraph, expr: ExprId, var: SymbolId) -> Option<ExprId> {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};

    let upoly = crate::poly::UniPoly::from_expr(graph, expr, var).ok()?;
    if upoly.degree() == 0 {
        return Some(expr);
    }

    let var_sym = graph.symbol(&graph.symbols.resolve(var)?);
    let mut cur = upoly.coeffs;
    while cur.len() > 1 && cur.last().is_some_and(|c| c.is_zero()) {
        cur.pop();
    }

    // Extract overall rational content so remaining primitive polynomial has integer coeffs & lc > 0
    let mut den_lcm = BigInt::one();
    let mut num_gcd = BigInt::zero();
    for c in &cur {
        den_lcm = den_lcm.lcm(c.denom());
    }
    let mut int_c: Vec<BigInt> = cur
        .iter()
        .map(|c| (c * BigRational::from_integer(den_lcm.clone())).to_integer())
        .collect();
    for ci in &int_c {
        num_gcd = num_gcd.gcd(&ci.abs());
    }
    if num_gcd.is_zero() {
        return Some(graph.integer(0));
    }
    let mut content = BigRational::new(num_gcd.clone(), den_lcm);
    if int_c.last().is_some_and(|c| c.is_negative()) {
        content = -content;
        int_c.iter_mut().for_each(|c| *c = -c.clone());
    }
    for ci in &mut int_c {
        *ci /= &num_gcd;
    }

    // Factor list: (primitive integer poly, multiplicity)
    let mut factored: Vec<(Vec<BigInt>, u32)> = Vec::new();

    // Extract x^k
    let mut x_mult = 0u32;
    while int_c.len() > 1 && int_c[0].is_zero() {
        x_mult += 1;
        int_c.remove(0);
    }
    if x_mult > 0 {
        factored.push((vec![BigInt::zero(), BigInt::one()], x_mult));
    }

    if int_c.len() > 1 {
        // Yun-style square-free decomposition in Q[x], normalized to primitive Z[x]
        let q_coeffs: Vec<BigRational> = int_c
            .iter()
            .cloned()
            .map(BigRational::from_integer)
            .collect();
        let mut sqfree_parts = Vec::new();
        for (sq_f, mult) in square_free_decomposition(&q_coeffs) {
            let prim = make_primitive_int_poly(&sq_f);
            for irr in factor_sqfree_primitive_int_poly(&prim) {
                sqfree_parts.push((irr, mult));
            }
        }
        // Merge equal factors
        for (f, m) in sqfree_parts {
            if let Some(pos) = factored.iter().position(|(ef, _)| ef == &f) {
                factored[pos].1 += m;
            } else {
                factored.push((f, m));
            }
        }
    }

    // Sort factors nicely (degree asc, then coeff tuple)
    factored.sort_by(|(fa, _), (fb, _)| {
        fa.len()
            .cmp(&fb.len())
            .then_with(|| fa.iter().rev().cmp(fb.iter().rev()))
    });

    let mut out_factors = Vec::new();
    if !content.is_one() {
        out_factors.push(crate::poly::expr_from_rational(graph, &content));
    }
    for (f_coeffs, mult) in factored {
        let f_expr = int_poly_to_expr(graph, &f_coeffs, var_sym);
        if mult == 1 {
            out_factors.push(f_expr);
        } else {
            out_factors.push(graph.pow(f_expr, graph.integer(mult as i64)));
        }
    }

    if out_factors.is_empty() {
        Some(graph.integer(1))
    } else if out_factors.len() == 1 {
        Some(out_factors[0])
    } else {
        Some(graph.mul(out_factors))
    }
}

fn int_poly_to_expr(graph: &ExprGraph, coeffs: &[num_bigint::BigInt], var_sym: ExprId) -> ExprId {
    use num_traits::{One as _, Signed as _, Zero as _};
    let mut terms: Vec<(usize, num_bigint::BigInt)> = Vec::new();
    for (deg, c) in coeffs.iter().enumerate().rev() {
        if !c.is_zero() {
            terms.push((deg, c.clone()));
        }
    }
    if terms.is_empty() {
        return graph.integer(0);
    }

    let build_abs = |deg: usize, c_abs: &num_bigint::BigInt| -> ExprId {
        let c_expr = crate::poly::expr_from_rational(
            graph,
            &num_rational::BigRational::from_integer(c_abs.clone()),
        );
        if deg == 0 {
            c_expr
        } else {
            let v_pow = if deg == 1 {
                var_sym
            } else {
                graph.pow(var_sym, graph.integer(deg as i64))
            };
            if c_abs.is_one() {
                v_pow
            } else {
                graph.mul([c_expr, v_pow])
            }
        }
    };

    let (d0, ref c0) = terms[0];
    let mut acc = if c0.is_negative() {
        graph.neg(build_abs(d0, &(-c0)))
    } else {
        build_abs(d0, c0)
    };
    for (d, c) in &terms[1..] {
        if c.is_negative() {
            acc = graph.sub(acc, build_abs(*d, &(-c)));
        } else {
            acc = graph.add([acc, build_abs(*d, c)]);
        }
    }
    acc
}

fn make_primitive_int_poly(q: &[num_rational::BigRational]) -> Vec<num_bigint::BigInt> {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One as _, Signed as _, Zero as _};
    let mut cur = q.to_vec();
    while cur.len() > 1 && cur.last().is_some_and(|c| c.is_zero()) {
        cur.pop();
    }
    let mut den_lcm = BigInt::one();
    for c in &cur {
        den_lcm = den_lcm.lcm(c.denom());
    }
    let mut ints: Vec<BigInt> = cur
        .iter()
        .map(|c| (c * BigRational::from_integer(den_lcm.clone())).to_integer())
        .collect();
    let mut g = BigInt::zero();
    for ci in &ints {
        g = g.gcd(&ci.abs());
    }
    if g.is_zero() {
        return vec![BigInt::zero()];
    }
    if ints.last().is_some_and(|c| c.is_negative()) {
        ints.iter_mut().for_each(|c| *c = -c.clone());
    }
    for ci in &mut ints {
        *ci /= &g;
    }
    ints
}

fn qpoly_gcd(
    a: &[num_rational::BigRational],
    b: &[num_rational::BigRational],
) -> Vec<num_rational::BigRational> {
    let pa = crate::poly::UniPoly { coeffs: a.to_vec() };
    let pb = crate::poly::UniPoly { coeffs: b.to_vec() };
    pa.gcd(&pb).map(|p| p.coeffs).unwrap_or_default()
}

fn qpoly_div(
    a: &[num_rational::BigRational],
    b: &[num_rational::BigRational],
) -> Vec<num_rational::BigRational> {
    let pa = crate::poly::UniPoly { coeffs: a.to_vec() };
    let pb = crate::poly::UniPoly { coeffs: b.to_vec() };
    pa.div_rem(&pb).map(|(q, _)| q.coeffs).unwrap_or_default()
}

fn square_free_decomposition(
    f: &[num_rational::BigRational],
) -> Vec<(Vec<num_rational::BigRational>, u32)> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::Zero as _;

    if f.len() <= 2 {
        return vec![(f.to_vec(), 1)];
    }
    let deriv: Vec<BigRational> = f
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, c)| c * BigRational::from_integer(BigInt::from(k)))
        .collect();
    let mut c = qpoly_gcd(f, &deriv);
    if c.len() <= 1 {
        return vec![(f.to_vec(), 1)];
    }
    let mut w = qpoly_div(f, &c);
    let mut i = 1u32;
    let mut res = Vec::new();
    while w.len() > 1 {
        let y = qpoly_gcd(&w, &c);
        let z = qpoly_div(&w, &y);
        if z.len() > 1 && !z.iter().all(|q| q.is_zero()) {
            res.push((z, i));
        }
        w = y.clone();
        c = qpoly_div(&c, &y);
        i += 1;
    }
    res
}

fn factor_sqfree_primitive_int_poly(f: &[num_bigint::BigInt]) -> Vec<Vec<num_bigint::BigInt>> {
    use num_bigint::BigInt;
    use num_traits::{One as _, Signed as _, ToPrimitive as _, Zero as _};

    let mut cur = f.to_vec();
    let mut factors = Vec::new();

    // Extract any x = 0 root first before divisor search on c0
    while cur.len() > 2 && cur[0].is_zero() {
        factors.push(vec![BigInt::zero(), BigInt::one()]);
        cur.remove(0);
    }
    if cur.len() == 2 && cur[0].is_zero() {
        factors.push(cur);
        return factors;
    }

    // 1. Deflate all rational linear factors (q*x - p)
    loop {
        let deg = cur.len().saturating_sub(1);
        if deg <= 1 {
            break;
        }
        let Some(c0) = cur[0].abs().to_u64() else {
            break;
        };
        let Some(cn) = cur[deg].abs().to_u64() else {
            break;
        };
        let p_divs = u64_divisors(c0, 64);
        let q_divs = u64_divisors(cn, 64);

        let mut found: Option<(i64, u64)> = None;
        'search: for &p in &p_divs {
            for &q in &q_divs {
                if <u64 as num::Integer>::gcd(&p, &q) != 1 {
                    continue;
                }
                for sign in [1i64, -1i64] {
                    let ps = sign * (p as i64);
                    // Homogeneous evaluation sum c_i * ps^i * q^(d-i) == 0
                    let mut sum = BigInt::zero();
                    let mut q_pow = BigInt::one();
                    let mut p_pow = BigInt::one();
                    let q_big = BigInt::from(q);
                    let p_big = BigInt::from(ps);
                    let mut terms_p = Vec::with_capacity(deg + 1);
                    for _ in 0..=deg {
                        terms_p.push(p_pow.clone());
                        p_pow *= &p_big;
                    }
                    for i in (0..=deg).rev() {
                        sum += &cur[i] * &terms_p[i] * &q_pow;
                        q_pow *= &q_big;
                    }
                    if sum.is_zero() {
                        found = Some((ps, q));
                        break 'search;
                    }
                }
            }
        }
        if let Some((ps, q)) = found {
            // Divisor factor is (q * x - ps), primitive
            let lin = vec![BigInt::from(-ps), BigInt::from(q)];
            if let Some(quot) = int_poly_div_exact(&cur, &lin) {
                factors.push(lin);
                cur = quot;
                if cur.last().is_some_and(|c| c.is_negative()) {
                    cur.iter_mut().for_each(|c| *c = -c.clone());
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let rem_deg = cur.len().saturating_sub(1);
    if rem_deg == 0 {
        return factors;
    }
    if rem_deg <= 3 {
        // No linear roots => quadratic and cubic over Z[x] are irreducible!
        factors.push(cur);
        return factors;
    }

    // 2. Modular Cantor-Zassenhaus + Hensel lift + subset recombination for deg >= 4
    factors.extend(berlekamp_zassenhaus_split(&cur));
    factors
}

fn u64_divisors(v: u64, cap: usize) -> Vec<u64> {
    if v == 0 {
        return vec![1];
    }
    let mut small = Vec::new();
    let mut large = Vec::new();
    let lim = (v as f64).sqrt() as u64 + 1;
    for d in 1..=lim {
        if d * d > v {
            break;
        }
        if v.is_multiple_of(d) {
            small.push(d);
            if d * d != v {
                large.push(v / d);
            }
            if small.len() + large.len() >= cap {
                break;
            }
        }
    }
    small.extend(large.into_iter().rev());
    small
}

fn int_poly_div_exact(
    a: &[num_bigint::BigInt],
    b: &[num_bigint::BigInt],
) -> Option<Vec<num_bigint::BigInt>> {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_traits::Zero as _;
    if b.is_empty() || b.last().is_some_and(|c| c.is_zero()) || a.len() < b.len() {
        return None;
    }
    let db = b.len() - 1;
    let lc = &b[db];
    let mut rem = a.to_vec();
    let mut q = vec![BigInt::zero(); a.len() - db];
    for k in (db..a.len()).rev() {
        let rk = rem[k].clone();
        if rk.is_zero() {
            continue;
        }
        let (quo, r) = rk.div_rem(lc);
        if !r.is_zero() {
            return None;
        }
        q[k - db] = quo.clone();
        for j in 0..=db {
            rem[k - db + j] -= &quo * &b[j];
        }
    }
    if rem.iter().all(|c| c.is_zero()) {
        while q.len() > 1 && q.last().is_some_and(|c| c.is_zero()) {
            q.pop();
        }
        Some(q)
    } else {
        None
    }
}

// ---------- Modular arithmetic mod prime p (u64) + Hensel lifting ----------

fn mod_pos(a: &num_bigint::BigInt, m: u64) -> u64 {
    use num_traits::ToPrimitive as _;
    let m_big = num_bigint::BigInt::from(m);
    let r = ((a % &m_big) + &m_big) % &m_big;
    r.to_u64().unwrap_or(0)
}

fn mod_inv_u64(a: u64, p: u64) -> u64 {
    mod_pow_u64(a % p, p - 2, p)
}

fn mod_pow_u64(mut base: u64, mut exp: u64, m: u64) -> u64 {
    let mut res = 1u128;
    let mut b = (base % m) as u128;
    let mu = m as u128;
    let _ = &mut base;
    while exp > 0 {
        if exp & 1 == 1 {
            res = (res * b) % mu;
        }
        exp >>= 1;
        if exp > 0 {
            b = (b * b) % mu;
        }
    }
    res as u64
}

fn fp_trim(p_coeffs: &mut Vec<u64>) {
    while p_coeffs.last().copied() == Some(0) {
        p_coeffs.pop();
    }
}

fn fp_monic(mut a: Vec<u64>, p: u64) -> Vec<u64> {
    fp_trim(&mut a);
    if let Some(&lc) = a.last() {
        let inv = mod_inv_u64(lc, p);
        for c in &mut a {
            *c = ((*c as u128 * inv as u128) % p as u128) as u64;
        }
    }
    a
}

fn fp_add(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    let mut out = vec![0u64; a.len().max(b.len())];
    for (i, &v) in a.iter().enumerate() {
        out[i] = v % p;
    }
    for (i, &v) in b.iter().enumerate() {
        out[i] = (out[i] + v) % p;
    }
    fp_trim(&mut out);
    out
}

fn fp_sub(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    let mut out = vec![0u64; a.len().max(b.len())];
    for (i, &v) in a.iter().enumerate() {
        out[i] = v % p;
    }
    for (i, &v) in b.iter().enumerate() {
        out[i] = (out[i] + p - (v % p)) % p;
    }
    fp_trim(&mut out);
    out
}

fn fp_mul(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len() - 1];
    let pu = p as u128;
    for (i, &ca) in a.iter().enumerate() {
        for (j, &cb) in b.iter().enumerate() {
            out[i + j] = ((out[i + j] as u128 + (ca as u128) * (cb as u128)) % pu) as u64;
        }
    }
    fp_trim(&mut out);
    out
}

fn fp_div_rem(a: &[u64], b: &[u64], p: u64) -> (Vec<u64>, Vec<u64>) {
    let mut rem = a.to_vec();
    fp_trim(&mut rem);
    let mut b_trim = b.to_vec();
    fp_trim(&mut b_trim);
    if rem.len() < b_trim.len() || b_trim.is_empty() {
        return (Vec::new(), rem);
    }
    let db = b_trim.len() - 1;
    let inv_lc = mod_inv_u64(b_trim[db], p) as u128;
    let pu = p as u128;
    let mut q = vec![0u64; rem.len() - db];
    for k in (db..rem.len()).rev() {
        let rk = rem[k] as u128;
        if rk == 0 {
            continue;
        }
        let coeff = (rk * inv_lc) % pu;
        q[k - db] = coeff as u64;
        for j in 0..=db {
            let sub = (coeff * b_trim[j] as u128) % pu;
            rem[k - db + j] = ((rem[k - db + j] as u128 + pu - sub) % pu) as u64;
        }
    }
    fp_trim(&mut q);
    fp_trim(&mut rem);
    (q, rem)
}

fn fp_gcd(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
    let mut x = a.to_vec();
    let mut y = b.to_vec();
    fp_trim(&mut x);
    fp_trim(&mut y);
    while !y.is_empty() {
        let (_, r) = fp_div_rem(&x, &y, p);
        x = y;
        y = r;
    }
    fp_monic(x, p)
}

fn fp_pow_mod_big(base: &[u64], exp: &num_bigint::BigUint, modulus: &[u64], p: u64) -> Vec<u64> {
    let mut res = vec![1u64];
    let bits = exp.bits();
    for bit in (0..bits).rev() {
        res = fp_div_rem(&fp_mul(&res, &res, p), modulus, p).1;
        if exp.bit(bit) {
            res = fp_div_rem(&fp_mul(&res, base, p), modulus, p).1;
        }
    }
    res
}

fn fp_edf(g: Vec<u64>, d: usize, p: u64, rng: &mut u64) -> Vec<Vec<u64>> {
    use num_bigint::BigUint;
    let deg_g = g.len().saturating_sub(1);
    if deg_g == d {
        return vec![g];
    }
    if deg_g < d || d == 0 {
        return Vec::new();
    }
    let pk_half = (BigUint::from(p).pow(d as u32) - 1u32) >> 1u32;
    let mut work = vec![g];
    let mut out = Vec::new();
    while let Some(poly) = work.pop() {
        let cur_deg = poly.len().saturating_sub(1);
        if cur_deg == d {
            out.push(poly);
            continue;
        }
        // Try random polynomials until split succeeds
        loop {
            let mut a = vec![0u64; cur_deg];
            for c in &mut a {
                *rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                *c = (*rng >> 16) % p;
            }
            fp_trim(&mut a);
            if a.len() <= 1 {
                continue;
            }
            let d1 = fp_gcd(&a, &poly, p);
            if d1.len() > 1 && d1.len() - 1 < cur_deg {
                let q = fp_div_rem(&poly, &d1, p).0;
                work.push(d1);
                work.push(fp_monic(q, p));
                break;
            }
            let b = fp_sub(&fp_pow_mod_big(&a, &pk_half, &poly, p), &[1], p);
            let d2 = fp_gcd(&b, &poly, p);
            if d2.len() > 1 && d2.len() - 1 < cur_deg {
                let q = fp_div_rem(&poly, &d2, p).0;
                work.push(d2);
                work.push(fp_monic(q, p));
                break;
            }
        }
    }
    out
}

fn fp_factor_monic(f: &[u64], p: u64) -> Vec<Vec<u64>> {
    use num_bigint::BigUint;
    let mut cur = fp_monic(f.to_vec(), p);
    let mut h = vec![0u64, 1u64]; // x
    let p_big = BigUint::from(p);
    let x_poly = vec![0u64, 1u64];
    let mut k = 1usize;
    let mut out = Vec::new();
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;

    while cur.len().saturating_sub(1) >= 2 * k {
        h = fp_pow_mod_big(&h, &p_big, &cur, p);
        let g = fp_gcd(&cur, &fp_sub(&h, &x_poly, p), p);
        if g.len() > 1 {
            cur = fp_monic(fp_div_rem(&cur, &g, p).0, p);
            out.extend(fp_edf(g, k, p, &mut rng));
        }
        k += 1;
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

// BigInt modular polynomial helpers for Hensel lifting f = prod h_i (mod p^e)
fn zp_trim(a: &mut Vec<num_bigint::BigInt>, m: &num_bigint::BigInt) {
    use num_traits::Zero as _;
    for c in a.iter_mut() {
        *c = ((&*c % m) + m) % m;
    }
    while a.last().is_some_and(|c| c.is_zero()) {
        a.pop();
    }
}

fn zp_mul(
    a: &[num_bigint::BigInt],
    b: &[num_bigint::BigInt],
    m: &num_bigint::BigInt,
) -> Vec<num_bigint::BigInt> {
    use num_bigint::BigInt;
    use num_traits::Zero as _;
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![BigInt::zero(); a.len() + b.len() - 1];
    for (i, ca) in a.iter().enumerate() {
        for (j, cb) in b.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    zp_trim(&mut out, m);
    out
}

fn fp_ext_gcd(a: &[u64], b: &[u64], p: u64) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
    let mut old_r = a.to_vec();
    let mut r = b.to_vec();
    let mut old_s = vec![1u64];
    let mut s = Vec::new();
    let mut old_t = Vec::new();
    let mut t = vec![1u64];
    fp_trim(&mut old_r);
    fp_trim(&mut r);

    while !r.is_empty() {
        let (q, rem) = fp_div_rem(&old_r, &r, p);
        old_r = r;
        r = rem;
        let new_s = fp_sub(&old_s, &fp_mul(&q, &s, p), p);
        old_s = s;
        s = new_s;
        let new_t = fp_sub(&old_t, &fp_mul(&q, &t, p), p);
        old_t = t;
        t = new_t;
    }
    if let Some(&lc) = old_r.last() {
        let inv = mod_inv_u64(lc, p);
        let scale = |v: Vec<u64>| -> Vec<u64> { fp_mul(&v, &[inv], p) };
        (scale(old_r), scale(old_s), scale(old_t))
    } else {
        (old_r, old_s, old_t)
    }
}

fn mod_inv_bigint(a: &num_bigint::BigInt, m: &num_bigint::BigInt) -> num_bigint::BigInt {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_traits::{One as _, Zero as _};
    let mut t = BigInt::zero();
    let mut newt = BigInt::one();
    let mut r = m.clone();
    let mut newr = ((a % m) + m) % m;
    while !newr.is_zero() {
        let (q, rem) = r.div_rem(&newr);
        let tmp_t = t - &q * &newt;
        t = newt;
        newt = tmp_t;
        r = newr;
        newr = rem;
    }
    ((t % m) + m) % m
}

fn hensel_lift_pair(
    f_monic: &[num_bigint::BigInt],
    a0: &[u64],
    b0: &[u64],
    p: u64,
    target_mod: &num_bigint::BigInt,
) -> (Vec<num_bigint::BigInt>, Vec<num_bigint::BigInt>) {
    use num_bigint::BigInt;
    let (_, s0, t0) = fp_ext_gcd(a0, b0, p); // s0*a0 + t0*b0 = 1 mod p
    let p_big = BigInt::from(p);
    let mut a: Vec<BigInt> = a0.iter().map(|&c| BigInt::from(c)).collect();
    let mut b: Vec<BigInt> = b0.iter().map(|&c| BigInt::from(c)).collect();
    let mut m = p_big.clone();

    while m < *target_mod {
        let next_m = &m * &p_big;
        let ab = zp_mul(&a, &b, &next_m);
        let max_len = f_monic.len().max(ab.len());
        let mut e_u64 = vec![0u64; max_len];
        for i in 0..max_len {
            let fi = f_monic.get(i).cloned().unwrap_or_default();
            let abi = ab.get(i).cloned().unwrap_or_default();
            let diff = ((fi - abi) % &next_m + &next_m) % &next_m;
            let q = diff / &m;
            e_u64[i] = mod_pos(&q, p);
        }
        fp_trim(&mut e_u64);

        // dB = (s0 * E) mod b0 (mod p); q_t = (s0 * E) / b0
        let se = fp_mul(&s0, &e_u64, p);
        let (q_s, db_p) = fp_div_rem(&se, b0, p);
        let te = fp_mul(&t0, &e_u64, p);
        let da_p = fp_div_rem(&fp_add(&te, &fp_mul(a0, &q_s, p), p), a0, p).1;

        let da_big: Vec<BigInt> = da_p.into_iter().map(BigInt::from).collect();
        let db_big: Vec<BigInt> = db_p.into_iter().map(BigInt::from).collect();

        let len_a = a.len().max(da_big.len());
        a.resize(len_a, BigInt::from(0));
        for (i, d) in da_big.into_iter().enumerate() {
            a[i] = (&a[i] + &m * d) % &next_m;
        }
        zp_trim(&mut a, &next_m);

        let len_b = b.len().max(db_big.len());
        b.resize(len_b, BigInt::from(0));
        for (i, d) in db_big.into_iter().enumerate() {
            b[i] = (&b[i] + &m * d) % &next_m;
        }
        zp_trim(&mut b, &next_m);

        m = next_m;
    }
    (a, b)
}

fn hensel_lift_all(
    f_monic: &[num_bigint::BigInt],
    factors_p: &[Vec<u64>],
    p: u64,
    target_mod: &num_bigint::BigInt,
) -> Vec<Vec<num_bigint::BigInt>> {
    if factors_p.len() == 1 {
        let mut fm = f_monic.to_vec();
        zp_trim(&mut fm, target_mod);
        return vec![fm];
    }
    let mid = factors_p.len() / 2;
    let left_p = &factors_p[..mid];
    let right_p = &factors_p[mid..];
    let mut a0 = vec![1u64];
    for lf in left_p {
        a0 = fp_mul(&a0, lf, p);
    }
    let mut b0 = vec![1u64];
    for rf in right_p {
        b0 = fp_mul(&b0, rf, p);
    }
    let (f_left, f_right) = hensel_lift_pair(f_monic, &a0, &b0, p, target_mod);
    let mut res = hensel_lift_all(&f_left, left_p, p, target_mod);
    res.extend(hensel_lift_all(&f_right, right_p, p, target_mod));
    res
}

fn berlekamp_zassenhaus_split(f: &[num_bigint::BigInt]) -> Vec<Vec<num_bigint::BigInt>> {
    use num::Integer as _;
    use num_bigint::BigInt;
    use num_traits::{Signed as _, Zero as _};

    let deg = f.len() - 1;
    let lc = &f[deg];
    let mut f_mods: Option<(u64, Vec<Vec<u64>>)> = None;

    for p in [
        5u64, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97, 101,
    ] {
        if mod_pos(lc, p) == 0 {
            continue;
        }
        let fp: Vec<u64> = f.iter().map(|c| mod_pos(c, p)).collect();
        let mut dfp: Vec<u64> = (1..fp.len())
            .map(|k| ((k as u64 % p) * fp[k]) % p)
            .collect();
        fp_trim(&mut dfp);
        let g = fp_gcd(&fp, &dfp, p);
        if g.len() > 1 {
            continue;
        }
        let parts = fp_factor_monic(&fp, p);
        f_mods = Some((p, parts));
        break;
    }

    let Some((p, parts)) = f_mods else {
        return vec![f.to_vec()];
    };
    if parts.len() <= 1 {
        return vec![f.to_vec()];
    }

    // Landau-Mignotte coefficient bound: lc * 2^(deg/2) * norm2(f)
    let mut sum_sq = BigInt::zero();
    for c in f {
        sum_sq += c * c;
    }
    let norm_ceil = sum_sq.sqrt() + 1;
    let bound = lc * (BigInt::from(1) << (deg / 2 + 2)) * norm_ceil;
    let two_bound = bound * 2 + 1;

    let p_big = BigInt::from(p);
    let mut mod_m = p_big.clone();
    while mod_m <= two_bound {
        mod_m *= &p_big;
    }

    let inv_lc = mod_inv_bigint(lc, &mod_m);
    let mut f_monic: Vec<BigInt> = f
        .iter()
        .map(|c| ((c * &inv_lc) % &mod_m + &mod_m) % &mod_m)
        .collect();
    zp_trim(&mut f_monic, &mod_m);

    let lifted = hensel_lift_all(&f_monic, &parts, p, &mod_m);
    let mut remaining_poly = f.to_vec();
    let mut active: Vec<Vec<BigInt>> = lifted;
    let mut irreducibles = Vec::new();
    let half_m = &mod_m / 2;

    let mut s = 1usize;
    while 2 * s <= active.len() {
        let mut found_subset: Option<Vec<usize>> = None;
        let mut cur_rem_factor = Vec::new();
        let mut comb = (0..s).collect::<Vec<usize>>();

        while let Some(cur_lc) = remaining_poly.last() {
            let mut prod = vec![cur_lc.clone()];
            for &idx in &comb {
                prod = zp_mul(&prod, &active[idx], &mod_m);
            }
            // Signed center mod M and take primitive part
            let mut cand: Vec<BigInt> = prod
                .into_iter()
                .map(|c| {
                    let r = ((&c % &mod_m) + &mod_m) % &mod_m;
                    if r > half_m { r - &mod_m } else { r }
                })
                .collect();
            while cand.len() > 1 && cand.last().is_some_and(|c| c.is_zero()) {
                cand.pop();
            }
            if cand.len() > 1 && cand.len() < remaining_poly.len() {
                let mut g = BigInt::zero();
                for c in &cand {
                    g = g.gcd(&c.abs());
                }
                if !g.is_zero() {
                    if cand.last().is_some_and(|c| c.is_negative()) {
                        g = -g;
                    }
                    for c in &mut cand {
                        *c /= &g;
                    }
                    if let Some(quot) = int_poly_div_exact(&remaining_poly, &cand) {
                        found_subset = Some(comb.clone());
                        cur_rem_factor = quot;
                        irreducibles.push(cand);
                        break;
                    }
                }
            }

            // Next k-combination
            let n = active.len();
            let mut i = s;
            while i > 0 {
                i -= 1;
                if comb[i] != i + n - s {
                    break;
                }
                if i == 0 && comb[0] == n - s {
                    i = usize::MAX;
                    break;
                }
            }
            if i == usize::MAX || (comb[i] == i + n - s) {
                break;
            }
            comb[i] += 1;
            for j in (i + 1)..s {
                comb[j] = comb[j - 1] + 1;
            }
        }

        if let Some(used) = found_subset {
            for idx in used.into_iter().rev() {
                active.remove(idx);
            }
            remaining_poly = cur_rem_factor;
            if remaining_poly.last().is_some_and(|c| c.is_negative()) {
                remaining_poly.iter_mut().for_each(|c| *c = -c.clone());
            }
            s = 1;
        } else {
            s += 1;
        }
    }

    if remaining_poly.len() > 1 {
        irreducibles.push(remaining_poly);
    }
    irreducibles
}

// ---------- Multivariate Polynomial Factorization over Q ----------

fn factor_multivariate_over_q(
    graph: &ExprGraph,
    expr: ExprId,
    syms: &[SymbolId],
) -> Option<ExprId> {
    use num::Integer as _;
    use num_traits::{One as _, Signed as _, Zero as _};

    let poly = mv_poly_from_expr(graph, expr)?;
    if poly.is_empty() {
        return Some(graph.integer(0));
    }

    // 1. Check all atoms are plain symbols
    let mut var_ids: Vec<ExprId> = Vec::new();
    for m in poly.keys() {
        for &(atom, _) in m {
            if !matches!(graph.get(atom).kind, ExprKind::Symbol(_)) {
                return None;
            }
            if !var_ids.contains(&atom) {
                var_ids.push(atom);
            }
        }
    }
    var_ids.sort_by_key(|&a| atom_sort_name(graph, a));

    if var_ids.len() <= 1 {
        if syms.len() == 1 {
            return factor_univariate_over_q(graph, expr, syms[0]);
        }
        return None;
    }

    // 2. Extract overall rational content and common monomial GCD
    let mut den_lcm = num_bigint::BigInt::one();
    let mut num_gcd = num_bigint::BigInt::zero();
    for c in poly.values() {
        den_lcm = den_lcm.lcm(c.denom());
    }
    for c in poly.values() {
        let int_val = (c * num_rational::BigRational::from_integer(den_lcm.clone())).to_integer();
        num_gcd = num_gcd.gcd(&int_val.abs());
    }
    let content = num_rational::BigRational::new(num_gcd, den_lcm);

    let mut common_exps: std::collections::BTreeMap<ExprId, u32> =
        std::collections::BTreeMap::new();
    for &v in &var_ids {
        let min_e = poly
            .keys()
            .map(|m| {
                m.iter()
                    .find(|&&(a, _)| a == v)
                    .map(|&(_, e)| e)
                    .unwrap_or(0)
            })
            .min()
            .unwrap_or(0);
        if min_e > 0 {
            common_exps.insert(v, min_e);
        }
    }

    // Divide out content & common monomials into primitive remaining poly `prim`
    let mut prim: MvPoly = std::collections::BTreeMap::new();
    for (m, c) in &poly {
        let mut nm = Vec::new();
        for &(a, e) in m {
            let sub = e - common_exps.get(&a).copied().unwrap_or(0);
            if sub > 0 {
                nm.push((a, sub));
            }
        }
        prim.insert(nm, c / &content);
    }

    // Check remaining variables in prim
    let mut prim_vars: Vec<ExprId> = Vec::new();
    for m in prim.keys() {
        for &(a, _) in m {
            if !prim_vars.contains(&a) {
                prim_vars.push(a);
            }
        }
    }
    prim_vars.sort_by_key(|&a| atom_sort_name(graph, a));

    let mut out_factors: Vec<ExprId> = Vec::new();
    if !content.is_one() {
        out_factors.push(crate::poly::expr_from_rational(graph, &content));
    }
    for (v, e) in common_exps {
        out_factors.push(if e == 1 {
            v
        } else {
            graph.pow(v, graph.integer(e as i64))
        });
    }

    if prim_vars.is_empty() {
        return Some(if out_factors.is_empty() {
            graph.integer(1)
        } else if out_factors.len() == 1 {
            out_factors[0]
        } else {
            graph.mul(out_factors)
        });
    }

    let prim_expr = mv_poly_to_expr(graph, &prim);
    if prim_vars.len() == 1 {
        if let ExprKind::Symbol(s_id) = graph.get(prim_vars[0]).kind {
            let f_prim = factor_univariate_over_q(graph, prim_expr, s_id)?;
            out_factors.push(f_prim);
            return Some(if out_factors.len() == 1 {
                out_factors[0]
            } else {
                graph.mul(out_factors)
            });
        }
    }

    // 3. Homogeneous bivariate exact factoring via dehomogenization (P(x, y) -> P(x, 1))
    if prim_vars.len() == 2 {
        let total_degs: Vec<u32> = prim
            .keys()
            .map(|m| m.iter().map(|&(_, e)| e).sum())
            .collect();
        let d0 = total_degs[0];
        if total_degs.iter().all(|&d| d == d0) && d0 >= 2 {
            let x_var = prim_vars[0];
            let y_var = prim_vars[1];
            let x_sym = match graph.get(x_var).kind {
                ExprKind::Symbol(s) => s,
                _ => return None,
            };
            let y_sym = match graph.get(y_var).kind {
                ExprKind::Symbol(s) => s,
                _ => return None,
            };
            let dehom = graph.substitute(prim_expr, y_sym, graph.integer(1));
            if let Ok(upoly) = crate::poly::UniPoly::from_expr(graph, dehom, x_sym) {
                let mut cur = upoly.coeffs;
                let prim_int = make_primitive_int_poly(&cur);
                let mut f_list: Vec<(Vec<num_bigint::BigInt>, u32)> = Vec::new();
                for (sq_f, mult) in square_free_decomposition(
                    &prim_int
                        .iter()
                        .cloned()
                        .map(num_rational::BigRational::from_integer)
                        .collect::<Vec<_>>(),
                ) {
                    let sq_prim = make_primitive_int_poly(&sq_f);
                    for irr in factor_sqfree_primitive_int_poly(&sq_prim) {
                        f_list.push((irr, mult));
                    }
                }
                let mut used_deg = 0u32;
                let mut hom_factors = Vec::new();
                f_list.sort_by(|(fa, _), (fb, _)| {
                    fa.len()
                        .cmp(&fb.len())
                        .then_with(|| fa.iter().rev().cmp(fb.iter().rev()))
                });
                for (fc, mult) in &f_list {
                    let df = (fc.len() - 1) as u32;
                    used_deg += df * mult;
                    let hom_f = hom_int_poly_to_expr(graph, fc, x_var, y_var);
                    if *mult == 1 {
                        hom_factors.push(hom_f);
                    } else {
                        hom_factors.push(graph.pow(hom_f, graph.integer(*mult as i64)));
                    }
                }
                if used_deg < d0 {
                    let rem_y = d0 - used_deg;
                    hom_factors.push(if rem_y == 1 {
                        y_var
                    } else {
                        graph.pow(y_var, graph.integer(rem_y as i64))
                    });
                }
                let _ = &mut cur;
                out_factors.extend(hom_factors);
                return Some(if out_factors.len() == 1 {
                    out_factors[0]
                } else {
                    graph.mul(out_factors)
                });
            }
        }
    }

    // 4. Four-term grouping check: A*C + A*D + B*C + B*D = (A + B)*(C + D)
    if prim.len() == 4 {
        let terms: Vec<(MonomialKey, num_rational::BigRational)> = prim.into_iter().collect();
        for &(i1, i2, j1, j2) in &[(0usize, 1usize, 2usize, 3usize), (0, 2, 1, 3), (0, 3, 1, 2)] {
            if let Some(grouped) =
                try_group_four_terms(graph, &terms[i1], &terms[i2], &terms[j1], &terms[j2])
            {
                out_factors.push(grouped);
                return Some(if out_factors.len() == 1 {
                    out_factors[0]
                } else {
                    graph.mul(out_factors)
                });
            }
        }
    }

    None
}

fn hom_int_poly_to_expr(
    graph: &ExprGraph,
    coeffs: &[num_bigint::BigInt],
    x_var: ExprId,
    y_var: ExprId,
) -> ExprId {
    use num_traits::{One as _, Signed as _, Zero as _};
    let d = coeffs.len() - 1;
    let mut terms = Vec::new();
    for (kx, c) in coeffs.iter().enumerate().rev() {
        if !c.is_zero() {
            terms.push((kx, d - kx, c.clone()));
        }
    }
    let build_abs = |kx: usize, ky: usize, c_abs: &num_bigint::BigInt| -> ExprId {
        let mut parts = Vec::new();
        if !c_abs.is_one() || (kx == 0 && ky == 0) {
            parts.push(crate::poly::expr_from_rational(
                graph,
                &num_rational::BigRational::from_integer(c_abs.clone()),
            ));
        }
        if kx == 1 {
            parts.push(x_var);
        } else if kx > 1 {
            parts.push(graph.pow(x_var, graph.integer(kx as i64)));
        }
        if ky == 1 {
            parts.push(y_var);
        } else if ky > 1 {
            parts.push(graph.pow(y_var, graph.integer(ky as i64)));
        }
        if parts.len() == 1 {
            parts[0]
        } else {
            graph.mul(parts)
        }
    };

    let (kx0, ky0, ref c0) = terms[0];
    let mut acc = if c0.is_negative() {
        graph.neg(build_abs(kx0, ky0, &(-c0)))
    } else {
        build_abs(kx0, ky0, c0)
    };
    for (kx, ky, c) in &terms[1..] {
        if c.is_negative() {
            acc = graph.sub(acc, build_abs(*kx, *ky, &(-c)));
        } else {
            acc = graph.add([acc, build_abs(*kx, *ky, c)]);
        }
    }
    acc
}

fn try_group_four_terms(
    graph: &ExprGraph,
    t0: &(MonomialKey, num_rational::BigRational),
    t1: &(MonomialKey, num_rational::BigRational),
    t2: &(MonomialKey, num_rational::BigRational),
    t3: &(MonomialKey, num_rational::BigRational),
) -> Option<ExprId> {
    use num_traits::Zero as _;
    // Factor t0 + t1 as g1 * (u0 + u1) and check if t2 + t3 is c * (u0 + u1)
    let gcd_mono = |a: &MonomialKey, b: &MonomialKey| -> MonomialKey {
        let mut out = Vec::new();
        for &(va, ea) in a {
            if let Some(&(_, eb)) = b.iter().find(|&&(vb, _)| vb == va) {
                let me = ea.min(eb);
                if me > 0 {
                    out.push((va, me));
                }
            }
        }
        out
    };
    let div_mono = |a: &MonomialKey, g: &MonomialKey| -> MonomialKey {
        let mut out = Vec::new();
        for &(va, ea) in a {
            let eg = g
                .iter()
                .find(|&&(vg, _)| vg == va)
                .map(|&(_, e)| e)
                .unwrap_or(0);
            if ea > eg {
                out.push((va, ea - eg));
            }
        }
        out
    };

    let g1 = gcd_mono(&t0.0, &t1.0);
    let g2 = gcd_mono(&t2.0, &t3.0);
    let u0 = div_mono(&t0.0, &g1);
    let u1 = div_mono(&t1.0, &g1);
    let v0 = div_mono(&t2.0, &g2);
    let v1 = div_mono(&t3.0, &g2);

    let check = |va: &MonomialKey,
                 vb: &MonomialKey,
                 c2: &num_rational::BigRational,
                 c3: &num_rational::BigRational|
     -> Option<num_rational::BigRational> {
        if *va == u0 && *vb == u1 {
            let r = c2 / &t0.1;
            if !r.is_zero() && (&t1.1 * &r) == *c3 {
                return Some(r);
            }
        }
        None
    };

    let (scale, swapped) = if let Some(r) = check(&v0, &v1, &t2.1, &t3.1) {
        (r, false)
    } else {
        let r = check(&v1, &v0, &t3.1, &t2.1)?;
        (r, true)
    };
    let _ = swapped;

    let mut f_left = MvPoly::new();
    f_left.insert(g1, num_rational::BigRational::from_integer(1.into()));
    f_left.insert(g2, scale);
    let mut f_right = MvPoly::new();
    f_right.insert(u0, t0.1.clone());
    f_right.insert(u1, t1.1.clone());

    Some(graph.mul([
        mv_poly_to_expr(graph, &f_left),
        mv_poly_to_expr(graph, &f_right),
    ]))
}
