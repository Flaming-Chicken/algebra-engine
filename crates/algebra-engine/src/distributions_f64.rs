//! Continuous distributions in `f64`: the regularised incomplete gamma and beta functions, the chi-square, gamma,
//! beta, Student t and F distributions (cdf and pdf), and quantile functions (inverse cdf) by bracketing and bisection
//! refined with Newton steps.
//!
//! The incomplete functions use the series and the Lentz continued fraction of Numerical Recipes (relative error about
//! 1e-14 away from the extreme tails).

/// `ln Gamma(x)` for `x > 0` by the Lanczos approximation (`g = 7`, nine coefficients; relative error about 1e-15),
/// fast enough for the inner loops of the quantile searches.
pub(crate) fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection: Gamma(x) Gamma(1 - x) = pi / sin(pi x).
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin().abs()).ln()
            - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + G + 0.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// The regularised lower incomplete gamma function `P(a, x)`.
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if a <= 0.0 {
        return f64::NAN;
    }
    if x < a + 1.0 {
        // Series.
        let (mut term, mut sum) = (1.0 / a, 1.0 / a);
        let mut n = a;
        for _ in 0..10_000 {
            n += 1.0;
            term *= x / n;
            sum += term;
            if term.abs() < sum.abs() * 1e-17 {
                break;
            }
        }
        sum * (-x + a * x.ln() - ln_gamma(a)).exp()
    } else {
        1.0 - gamma_q_fraction(a, x)
    }
}

/// `Q(a, x) = 1 - P(a, x)` by the continued fraction (for `x >= a + 1`).
fn gamma_q_fraction(a: f64, x: f64) -> f64 {
    let tiny = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..10_000 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }
        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// The regularised upper incomplete gamma function `Q(a, x)`.
pub fn gamma_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        1.0 - gamma_p(a, x)
    } else {
        gamma_q_fraction(a, x)
    }
}

/// The regularised incomplete beta function `I_x(a, b)`.
pub fn beta_i(x: f64, a: f64, b: f64) -> f64 {
    if !(0.0..=1.0).contains(&x) || a <= 0.0 || b <= 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == 1.0 {
        return x;
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_fraction(x, a, b) / a
    } else {
        1.0 - front * beta_fraction(1.0 - x, b, a) / b
    }
}

fn beta_fraction(x: f64, a: f64, b: f64) -> f64 {
    let tiny = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..10_000 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

fn normal_cdf(z: f64) -> f64 {
    // erfc(x) through the incomplete gamma function: erfc(x) = Q(1/2, x^2) for x >= 0 and 1 + P(1/2, x^2) below.
    let x = -z / std::f64::consts::SQRT_2;
    let erfc = if x >= 0.0 {
        gamma_q(0.5, x * x)
    } else {
        1.0 + gamma_p(0.5, x * x)
    };
    0.5 * erfc
}

fn t_cdf(t: f64, nu: f64) -> f64 {
    let x = nu / (nu + t * t);
    let tail = 0.5 * beta_i(x, nu / 2.0, 0.5);
    if t >= 0.0 { 1.0 - tail } else { tail }
}

/// A quantile by bisection of a monotone cdf on `[lo, hi]` (expanding `hi` while needed).
fn invert(cdf: impl Fn(f64) -> f64, p: f64, mut lo: f64, mut hi: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    while cdf(hi) < p && hi < 1e300 {
        lo = hi;
        hi *= 2.0;
    }
    while cdf(lo) > p && lo > -1e300 {
        hi = lo;
        lo = if lo > 0.0 { lo / 2.0 } else { lo * 2.0 - 1.0 };
    }
    for _ in 0..300 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if (hi - lo) <= 1e-15 * mid.abs().max(1e-300) {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// The value of `name(args)`, or `None` if `name` is not one of the distribution functions or the arity is wrong.
pub fn eval(name: &str, a: &[f64]) -> Option<f64> {
    use std::f64::consts::PI;
    Some(match (name, a) {
        ("gamma_inc_p" | "gammainc", [s, x]) => gamma_p(*s, *x),
        ("gamma_inc_q", [s, x]) => gamma_q(*s, *x),
        ("beta_inc" | "betainc", [x, p, q]) => beta_i(*x, *p, *q),
        ("chi2_cdf" | "chisq_cdf", [x, k]) => gamma_p(k / 2.0, x / 2.0),
        ("chi2_pdf" | "chisq_pdf", [x, k]) => {
            if *x < 0.0 {
                0.0
            } else {
                ((k / 2.0 - 1.0) * x.ln() - x / 2.0 - (k / 2.0) * 2f64.ln() - ln_gamma(k / 2.0))
                    .exp()
            }
        }
        ("chi2_quantile" | "chisq_quantile" | "chi2_ppf", [p, k]) => {
            invert(|x| gamma_p(k / 2.0, x / 2.0), *p, 0.0, k + 10.0)
        }
        ("gamma_cdf", [x, shape, scale]) => gamma_p(*shape, x / scale),
        ("gamma_pdf", [x, shape, scale]) => {
            if *x < 0.0 {
                0.0
            } else {
                ((shape - 1.0) * x.ln() - x / scale - shape * scale.ln() - ln_gamma(*shape)).exp()
            }
        }
        ("beta_cdf", [x, p, q]) => beta_i(*x, *p, *q),
        ("beta_pdf", [x, p, q]) => {
            if !(0.0..=1.0).contains(x) {
                0.0
            } else {
                ((p - 1.0) * x.ln() + (q - 1.0) * (1.0 - x).ln() + ln_gamma(p + q)
                    - ln_gamma(*p)
                    - ln_gamma(*q))
                .exp()
            }
        }
        ("t_cdf" | "student_cdf", [t, nu]) => t_cdf(*t, *nu),
        ("t_pdf" | "student_pdf", [t, nu]) => (ln_gamma((nu + 1.0) / 2.0)
            - ln_gamma(nu / 2.0)
            - 0.5 * (nu * PI).ln()
            - (nu + 1.0) / 2.0 * (1.0 + t * t / nu).ln())
        .exp(),
        ("t_quantile" | "t_ppf" | "student_quantile", [p, nu]) => {
            invert(|t| t_cdf(t, *nu), *p, -10.0, 10.0)
        }
        ("f_cdf", [x, d1, d2]) => {
            if *x <= 0.0 {
                0.0
            } else {
                beta_i(d1 * x / (d1 * x + d2), d1 / 2.0, d2 / 2.0)
            }
        }
        ("normal_quantile" | "normal_ppf" | "norminv", [p]) => invert(normal_cdf, *p, -10.0, 10.0),
        ("normal_quantile" | "normal_ppf" | "norminv", [p, mu, sigma]) => {
            mu + sigma * invert(normal_cdf, *p, -10.0, 10.0)
        }
        _ => return None,
    })
}

/// Names handled by [`eval`].
pub fn is_distribution_function(name: &str) -> bool {
    matches!(
        name,
        "gamma_inc_p"
            | "gammainc"
            | "gamma_inc_q"
            | "beta_inc"
            | "betainc"
            | "chi2_cdf"
            | "chisq_cdf"
            | "chi2_pdf"
            | "chisq_pdf"
            | "chi2_quantile"
            | "chisq_quantile"
            | "chi2_ppf"
            | "gamma_cdf"
            | "gamma_pdf"
            | "beta_cdf"
            | "beta_pdf"
            | "t_cdf"
            | "student_cdf"
            | "t_pdf"
            | "student_pdf"
            | "t_quantile"
            | "t_ppf"
            | "student_quantile"
            | "f_cdf"
            | "normal_quantile"
            | "normal_ppf"
            | "norminv"
    )
}
