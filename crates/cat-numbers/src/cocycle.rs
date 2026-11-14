//! Bitmask 2-cocycle & 3-cocycle SIMD evaluation engine.
//!
//! Provides nanosecond Clifford geometric product, exterior wedge product,
//! interior contractions, and octonion associator computations.

use cat_core::scalar::CocycleMultivector;

/// High-performance algebraic multivector operations.
pub struct CocycleEngine;

impl CocycleEngine {
    /// Compute exterior wedge product: A \wedge B = \sum_{r, s} \langle \langle A \rangle_r \langle B \rangle_s \rangle_{r+s}.
    pub fn wedge(a: &CocycleMultivector, b: &CocycleMultivector) -> CocycleMultivector {
        let mut out = [0.0; 8];
        for u in 0..8u8 {
            let cu = a.blades[u as usize];
            if cu == 0.0 {
                continue;
            }
            let grade_u = u.count_ones();
            for v in 0..8u8 {
                let cv = b.blades[v as usize];
                if cv == 0.0 {
                    continue;
                }
                let grade_v = v.count_ones();
                // Blades are linearly independent in wedge product only if they share no basis factors
                if (u & v) == 0 && (grade_u + grade_v) == (u ^ v).count_ones() {
                    let target = u ^ v;
                    if (target as usize) < 8 {
                        // Count swaps to bring indices in order
                        let mut swaps = 0u32;
                        for i in 0..8u8 {
                            if (v & (1 << i)) != 0 {
                                let higher = u & !((1 << (i + 1)) - 1);
                                swaps += higher.count_ones();
                            }
                        }
                        let sign = if swaps % 2 == 1 { -1.0 } else { 1.0 };
                        out[target as usize] += cu * cv * sign;
                    }
                }
            }
        }
        CocycleMultivector {
            kind: a.kind,
            blades: out,
        }
    }

    /// Compute commutator bracket: [A, B] = 1/2 (A B - B A).
    pub fn commutator(a: &CocycleMultivector, b: &CocycleMultivector) -> CocycleMultivector {
        let ab = a.geometric_product(b);
        let ba = b.geometric_product(a);
        ab.sub(&ba).scale(0.5)
    }

    /// Compute anti-commutator: {A, B} = 1/2 (A B + B A).
    pub fn anticommutator(a: &CocycleMultivector, b: &CocycleMultivector) -> CocycleMultivector {
        let ab = a.geometric_product(b);
        let ba = b.geometric_product(a);
        ab.add(&ba).scale(0.5)
    }

    /// Compute 3-cocycle associator: \phi(A, B, C) = (A B) C - A (B C).
    /// Vanishes for associative algebras (Complex, Quaternion, Clifford), non-zero for Octonions!
    pub fn associator(
        a: &CocycleMultivector,
        b: &CocycleMultivector,
        c: &CocycleMultivector,
    ) -> CocycleMultivector {
        let ab_c = a.geometric_product(b).geometric_product(c);
        let a_bc = a.geometric_product(&b.geometric_product(c));
        ab_c.sub(&a_bc)
    }
}
