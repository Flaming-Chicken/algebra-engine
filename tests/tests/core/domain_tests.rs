use algebra_core::Domain;

#[test]
fn test_domain_subdomain_hierarchy() {
    assert!(Domain::Integers.is_subdomain_of(&Domain::Reals));
    assert!(Domain::Reals.is_subdomain_of(&Domain::Complex));
    assert!(!Domain::Complex.is_subdomain_of(&Domain::Reals));
}

#[test]
fn test_non_commutativity() {
    let mat_domain = Domain::Matrix {
        rows: 3,
        cols: 3,
        element_domain: Box::new(Domain::Reals),
    };
    assert!(!mat_domain.is_commutative_mul());
    assert!(Domain::Reals.is_commutative_mul());
}

#[test]
fn domain_categories_are_mathematically_correct() {
    use algebra_core::domain::DomainCategory as C;
    // N has no negatives: a semiring.
    assert_eq!(Domain::Naturals.category(), C::CommutativeSemiring);
    // Z/nZ is a field exactly when n is prime.
    for (n, field) in [
        (2u64, true),
        (3, true),
        (4, false),
        (6, false),
        (7, true),
        (9, false),
        (97, true),
        (100, false),
    ] {
        let want = if field { C::Field } else { C::CommutativeRing };
        assert_eq!(Domain::Modulo { modulus: n }.category(), want, "Z/{n}Z");
    }
    // The adele ring has zero divisors.
    assert_eq!(Domain::Adeles.category(), C::CommutativeRing);
    // C is algebraically closed; Q, R are fields that are not.
    assert_eq!(Domain::Complex.category(), C::AlgebraicClosedField);
    assert_eq!(Domain::Rationals.category(), C::Field);
    assert_eq!(Domain::Reals.category(), C::Field);
}
