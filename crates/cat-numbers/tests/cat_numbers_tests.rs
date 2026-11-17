use cat_core::scalar::CocycleMultivector;
use cat_numbers::*;
use std::time::Instant;

#[test]
fn test_butler_portugal_riemann_canonicalization() {
    let sym = TensorSymmetry::riemann();

    // R_{1023} should canonicalize to -R_{0123}
    let res1 = ButlerPortugal::canonicalize_4([1, 0, 2, 3], &sym);
    assert_eq!(res1.indices, [0, 1, 2, 3]);
    assert_eq!(res1.sign, -1);

    // R_{2301} should canonicalize to +R_{0123} (pair interchange)
    let res2 = ButlerPortugal::canonicalize_4([2, 3, 0, 1], &sym);
    assert_eq!(res2.indices, [0, 1, 2, 3]);
    assert_eq!(res2.sign, 1);

    // R_{0023} has first two indices identical -> vanishes identically (sign == 0)
    let res3 = ButlerPortugal::canonicalize_4([0, 0, 2, 3], &sym);
    assert_eq!(
        res3.sign, 0,
        "Antisymmetric tensor with identical indices must vanish"
    );

    // Garnir Young Projector for First Bianchi Identity:
    // When indices are standard [0, 1, 2, 3], returns single term with sign +1
    let proj = ButlerPortugal::garnir_bianchi_project([0, 1, 2, 3], &sym);
    assert_eq!(proj.len(), 1);
    assert_eq!(proj[0].0, [0, 1, 2, 3]);
    assert_eq!(proj[0].1, 1);

    // Microbenchmark: verify latency <= 20 microseconds
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = ButlerPortugal::canonicalize_4([3, 2, 1, 0], &sym);
    }
    let elapsed = start.elapsed();
    let per_op = elapsed.as_micros() as f64 / 1000.0;
    assert!(
        per_op <= 20.0,
        "Butler-Portugal canonicalization took {:.3} µs, must be <= 20 µs",
        per_op
    );
}

#[test]
fn test_divided_power_pd_algebra_lucas() {
    // \gamma_3(x) * \gamma_2(x) = \binom{5}{2} \gamma_5(x) = 10 \gamma_5(x)
    // Modulo p=3: 10 mod 3 = 1
    let dp1 = DividedPowerAtom::new(3, 3, 1);
    let dp2 = DividedPowerAtom::new(2, 3, 1);
    let prod = dp1
        .mul(&dp2)
        .expect("Mul in same characteristic must succeed");
    assert_eq!(prod.degree, 5);
    assert_eq!(prod.coeff, 1, "10 mod 3 == 1");

    // Modulo p=5: 10 mod 5 = 0 (vanishes modulo characteristic!)
    let dp_p5_1 = DividedPowerAtom::new(3, 5, 1);
    let dp_p5_2 = DividedPowerAtom::new(2, 5, 1);
    let prod5 = dp_p5_1.mul(&dp_p5_2).expect("Mul must succeed");
    assert_eq!(prod5.degree, 5);
    assert_eq!(prod5.coeff, 0, "10 mod 5 == 0");

    // Lucas's theorem: \binom{1000}{357} mod 7
    let binom = lucas_binomial_mod_p(1000, 357, 7);
    assert!(binom < 7);
}

#[test]
fn test_lyndon_words_and_lie_brackets() {
    // Single generators are Lyndon words
    assert!(LyndonWord::is_lyndon(&[0]));
    assert!(LyndonWord::is_lyndon(&[1]));

    // [0, 1] is Lyndon: suffixes are [1], and [0, 1] < [1]
    assert!(LyndonWord::is_lyndon(&[0, 1]));

    // [1, 0] is NOT Lyndon: suffix [0] is smaller than [1, 0]
    assert!(!LyndonWord::is_lyndon(&[1, 0]));

    // Shirshov factorization of [0, 1] -> [0] and [1]
    let w = LyndonWord::generator(0);
    let w1 = LyndonWord::generator(1);
    let w01 = LyndonWord(smallvec::smallvec![0, 1]);
    let (u, v) = w01
        .shirshov_factorization()
        .expect("Factorization of [0, 1] must succeed");
    assert_eq!(u, w);
    assert_eq!(v, w1);

    // Lie bracket tree construction
    let tree = LieBracketTree::from_lyndon(&w01);
    assert_eq!(
        tree,
        LieBracketTree::Bracket(
            Box::new(LieBracketTree::Generator(0)),
            Box::new(LieBracketTree::Generator(1))
        )
    );
}

#[test]
fn test_cocycle_wedge_and_associator() {
    // In Cl(3, 0): e_1 ^ e_2 = e_12
    let mut b1 = [0.0; 8];
    b1[1] = 1.0;
    let mut b2 = [0.0; 8];
    b2[2] = 1.0;
    let e1 = CocycleMultivector::cl3(b1);
    let e2 = CocycleMultivector::cl3(b2);

    let wedge_12 = CocycleEngine::wedge(&e1, &e2);
    // Mask 0b011 = 3 -> e_12
    assert_eq!(wedge_12.blades[3], 1.0);
    assert_eq!(wedge_12.blades[0], 0.0);

    // e_1 ^ e_1 = 0
    let wedge_11 = CocycleEngine::wedge(&e1, &e1);
    assert!(wedge_11.is_zero());

    // Quaternions are associative: associator must be zero
    let q_i = CocycleMultivector::quaternion(0.0, 1.0, 0.0, 0.0);
    let q_j = CocycleMultivector::quaternion(0.0, 0.0, 1.0, 0.0);
    let q_k = CocycleMultivector::quaternion(0.0, 0.0, 0.0, 1.0);
    let assoc_q = CocycleEngine::associator(&q_i, &q_j, &q_k);
    assert!(assoc_q.is_zero(), "Quaternion associator must be zero");

    // Octonions: non-associative for basis elements e_1, e_2, e_4
    let mut o1 = [0.0; 8];
    o1[1] = 1.0;
    let mut o2 = [0.0; 8];
    o2[2] = 1.0;
    let mut o4 = [0.0; 8];
    o4[4] = 1.0;
    let oct1 = CocycleMultivector::octonion(o1);
    let oct2 = CocycleMultivector::octonion(o2);
    let oct4 = CocycleMultivector::octonion(o4);
    let assoc_oct = CocycleEngine::associator(&oct1, &oct2, &oct4);
    assert!(!assoc_oct.is_zero(), "Octonion associator must be non-zero");
}

#[test]
fn test_butler_portugal_rank_8_symmetric_and_antisymmetric() {
    use cat_numbers::butler_portugal::TensorSymmetryN;

    let sym8 = TensorSymmetryN::symmetric(8);
    let res_sym = ButlerPortugal::canonicalize_n(&[7, 5, 3, 1, 6, 4, 2, 0], &sym8, &[]);
    assert_eq!(res_sym.sign, 1);
    assert_eq!(res_sym.indices, vec![0, 1, 2, 3, 4, 5, 6, 7]);

    let anti8 = TensorSymmetryN::antisymmetric(8);
    // Repeated index -> immediate 0 short-circuit
    let res_zero = ButlerPortugal::canonicalize_n(&[0, 1, 2, 3, 4, 5, 6, 6], &anti8, &[]);
    assert_eq!(res_zero.sign, 0);

    // Odd transposition on rank-8 antisymmetric tensor
    let res_odd = ButlerPortugal::canonicalize_n(&[1, 0, 2, 3, 4, 5, 6, 7], &anti8, &[]);
    assert_eq!(res_odd.sign, -1);
    assert_eq!(res_odd.indices, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}
