//! Proofs of Zeilberger recurrences for all `n` (guess the certificate, check it symbolically).

use algebra_engine::gosper::QPoly;
use algebra_engine::guess::Recurrence;
use algebra_engine::zeilberger::zeilberger;
use algebra_engine::zeilberger_proof::{Poly2, SymbolicTerm, check_certificate, prove_recurrence};

fn n_minus_k() -> Poly2 {
    Poly2::from_terms(&[(0, 1, 1), (1, 0, -1)])
}
fn k_plus_1() -> Poly2 {
    Poly2::from_terms(&[(1, 0, 1), (0, 0, 1)])
}
fn n_plus_1() -> Poly2 {
    Poly2::from_terms(&[(0, 1, 1), (0, 0, 1)])
}
fn n_plus_1_minus_k() -> Poly2 {
    Poly2::from_terms(&[(0, 1, 1), (0, 0, 1), (1, 0, -1)])
}
fn n_plus_k_plus_1() -> Poly2 {
    Poly2::from_terms(&[(0, 1, 1), (1, 0, 1), (0, 0, 1)])
}

/// C(n, k).
fn binomial() -> SymbolicTerm {
    SymbolicTerm {
        ratio_k: (n_minus_k(), k_plus_1()),
        ratio_n: (n_plus_1(), n_plus_1_minus_k()),
    }
}

/// C(n, k)^2.
fn squared_binomial() -> SymbolicTerm {
    SymbolicTerm {
        ratio_k: (n_minus_k().mul(&n_minus_k()), k_plus_1().mul(&k_plus_1())),
        ratio_n: (
            n_plus_1().mul(&n_plus_1()),
            n_plus_1_minus_k().mul(&n_plus_1_minus_k()),
        ),
    }
}

/// C(n, k) C(n + k, k).
fn delannoy() -> SymbolicTerm {
    SymbolicTerm {
        ratio_k: (
            n_minus_k().mul(&n_plus_k_plus_1()),
            k_plus_1().mul(&k_plus_1()),
        ),
        ratio_n: (n_plus_k_plus_1(), n_plus_1_minus_k()),
    }
}

fn found(term: &SymbolicTerm, samples: &[i64], order: usize, degree: usize) -> Recurrence {
    zeilberger(&term.to_term(), samples, order, degree)
        .unwrap()
        .expect("a recurrence")
        .recurrence
}

#[test]
fn the_binomial_row_sum_recurrence_is_proved_for_all_n() {
    let term = binomial();
    let rec = found(&term, &(2..10).collect::<Vec<_>>(), 1, 0);
    let proof = prove_recurrence(&term, &rec, &(3..16).collect::<Vec<_>>()).expect("proof");
    // The certificate is G = F * R with R = -k / (n + 1 - k) up to the normalisation of U / V.
    assert!(!proof.certificate_denominator.is_zero());
    assert_eq!(proof.recurrence, rec);
    // The symbolic check is not vacuous: the proved certificate passes it, a perturbed one fails.
    let (u, v) = (&proof.certificate_numerator, &proof.certificate_denominator);
    check_certificate(&term, &rec, u, v).expect("the proved certificate passes");
    let perturbed = u.add(&Poly2::from_terms(&[(0, 0, 1)]));
    assert!(check_certificate(&term, &rec, &perturbed, v).is_err());
    // Nor is a wrong recurrence accepted with the certificate of the right one.
    let wrong = Recurrence {
        coefficients: vec![QPoly::from_ints(&[-3]), QPoly::from_ints(&[1])],
    };
    assert!(check_certificate(&term, &wrong, u, v).is_err());
}

#[test]
fn squared_binomials_and_delannoy_numbers_are_proved_for_all_n() {
    let term = squared_binomial();
    let rec = found(&term, &(2..12).collect::<Vec<_>>(), 1, 1);
    prove_recurrence(&term, &rec, &(3..20).collect::<Vec<_>>()).expect("C(n,k)^2 proof");

    let term = delannoy();
    let rec = found(&term, &(2..14).collect::<Vec<_>>(), 2, 1);
    prove_recurrence(&term, &rec, &(3..22).collect::<Vec<_>>()).expect("Delannoy proof");
}

#[test]
fn a_wrong_recurrence_is_not_proved() {
    // S(n+1) - 3 S(n) = 0 is false for the binomial row sums.
    let wrong = Recurrence {
        coefficients: vec![QPoly::from_ints(&[-3]), QPoly::from_ints(&[1])],
    };
    assert!(prove_recurrence(&binomial(), &wrong, &(3..16).collect::<Vec<_>>()).is_err());
    // The right coefficients attached to the wrong term are rejected too.
    let right = Recurrence {
        coefficients: vec![QPoly::from_ints(&[-2]), QPoly::from_ints(&[1])],
    };
    assert!(prove_recurrence(&squared_binomial(), &right, &(3..16).collect::<Vec<_>>()).is_err());
}

#[test]
fn bivariate_polynomial_arithmetic_and_shifts() {
    // (n - k)(n + k) = n^2 - k^2, and shifts compose as substitutions.
    let product = n_minus_k().mul(&Poly2::from_terms(&[(0, 1, 1), (1, 0, 1)]));
    assert_eq!(product, Poly2::from_terms(&[(0, 2, 1), (2, 0, -1)]));
    assert_eq!(
        product.shift_n(1),
        Poly2::from_terms(&[(0, 2, 1), (0, 1, 2), (0, 0, 1), (2, 0, -1)])
    );
    assert_eq!(
        product.shift_k(2),
        Poly2::from_terms(&[(0, 2, 1), (2, 0, -1), (1, 0, -4), (0, 0, -4)])
    );
}
