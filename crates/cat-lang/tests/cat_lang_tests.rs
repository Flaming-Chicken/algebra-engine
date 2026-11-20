use cat_core::dimensions::DimVector;
use cat_lang::*;
use std::time::Instant;

#[test]
fn test_colored_operad_dimensional_elaboration() {
    let mut elab = Elaborator::new();
    elab.bind("length", DimVector::length());
    elab.bind("time", DimVector::time());

    // Expression: length / time => velocity
    let expr = CatExpr::BinaryOp {
        op: "/".into(),
        lhs: Box::new(CatExpr::Variable("length".into())),
        rhs: Box::new(CatExpr::Variable("time".into())),
    };

    let dim = elab
        .infer_dimension(&expr)
        .expect("Dimension inference must succeed");
    assert_eq!(dim, DimVector::velocity());

    // Inconsistent addition: length + time => Error
    let bad_add = CatExpr::BinaryOp {
        op: "+".into(),
        lhs: Box::new(CatExpr::Variable("length".into())),
        rhs: Box::new(CatExpr::Variable("time".into())),
    };
    let err = elab.infer_dimension(&bad_add);
    assert!(
        err.is_err(),
        "Adding length and time must fail dimensional elaboration"
    );
}

#[test]
fn test_pad_to_cat_promotion() {
    let pad = CatPadNotebook {
        cells: vec![
            PadCell::Markdown("# Simulation".into()),
            PadCell::Slider {
                name: "mass".into(),
                min: 0.0,
                max: 100.0,
                value: 50.0,
            },
            PadCell::Code {
                source: "mass * 9.81".into(),
                expr: CatExpr::BinaryOp {
                    op: "*".into(),
                    lhs: Box::new(CatExpr::Variable("mass".into())),
                    rhs: Box::new(CatExpr::Number(9.81)),
                },
                last_estimate: Some("490.5 N".into()),
            },
        ],
    };

    let script = pad.promote_to_script();
    assert_eq!(script.statements.len(), 2);
    match &script.statements[0] {
        CatStatement::Let { name, value, .. } => {
            assert_eq!(name, "mass");
            assert_eq!(*value, CatExpr::Number(50.0));
        }
        _ => panic!("Expected Let statement from promoted slider"),
    }
}

#[test]
fn test_binary_arena_persistence_roundtrip() {
    let script = CatScriptAst {
        statements: vec![CatStatement::Let {
            name: "speed_of_light".into(),
            type_ann: Some(CatTypeAnnotation::Dimension(DimVector::velocity())),
            value: CatExpr::Number(299_792_458.0),
        }],
    };

    let bytes = CatBinary::save_script(&script).expect("Serialization to .catb must succeed");
    let restored: CatScriptAst =
        CatBinary::load_script(&bytes).expect("Deserialization from .catb must succeed");
    assert_eq!(restored, script);
}

#[test]
fn test_interactive_keystroke_to_estimate_latency() {
    let start = Instant::now();
    let mut elab = Elaborator::new();
    elab.bind("x", DimVector::length());

    let expr = CatExpr::BinaryOp {
        op: "*".into(),
        lhs: Box::new(CatExpr::Variable("x".into())),
        rhs: Box::new(CatExpr::Number(2.0)),
    };
    let _ = elab.infer_dimension(&expr);
    let elapsed = start.elapsed();

    // Verify keystroke to estimate latency <= 8 ms
    assert!(
        elapsed.as_millis() <= 8,
        "Interactive elaboration took {} ms, must be <= 8 ms",
        elapsed.as_millis()
    );
}

fn var(n: &str) -> Box<CatExpr> {
    Box::new(CatExpr::Variable(n.into()))
}

fn num(v: f64) -> Box<CatExpr> {
    Box::new(CatExpr::Number(v))
}

fn bin(op: &str, l: Box<CatExpr>, r: Box<CatExpr>) -> CatExpr {
    CatExpr::BinaryOp {
        op: op.into(),
        lhs: l,
        rhs: r,
    }
}

fn call(f: &str, args: Vec<CatExpr>) -> CatExpr {
    CatExpr::Call {
        func: f.into(),
        args,
    }
}

#[test]
fn powers_scale_the_dimension_by_the_exponent() {
    let mut elab = Elaborator::new();
    elab.bind("l", DimVector::length());
    elab.bind("t", DimVector::time());
    // l^2 is an area, l^3 a volume, t^-1 a frequency.
    let area = elab.infer_dimension(&bin("^", var("l"), num(2.0))).unwrap();
    assert_eq!(area, DimVector::length() * DimVector::length());
    let inv = elab
        .infer_dimension(&bin("^", var("t"), num(-1.0)))
        .unwrap();
    assert_eq!(inv, DimVector::DIMENSIONLESS / DimVector::time());
    // A fractional exponent, written as a quotient: l^(1/2) is the square root of a length.
    let half = elab
        .infer_dimension(&bin("^", var("l"), Box::new(bin("/", num(1.0), num(2.0)))))
        .unwrap();
    assert_eq!(half, DimVector::length().sqrt().unwrap());
    // A pure number to any power stays a pure number.
    assert!(
        elab.infer_dimension(&bin("^", num(2.0), num(0.5)))
            .unwrap()
            .is_dimensionless()
    );
    // The exponent of a quantity with units must be a literal.
    assert!(elab.infer_dimension(&bin("^", var("l"), var("t"))).is_err());
    assert!(
        elab.infer_dimension(&bin("^", var("l"), num(0.123_456_789)))
            .is_err()
    );
}

#[test]
fn functions_respect_dimensions() {
    let mut elab = Elaborator::new();
    elab.bind("l", DimVector::length());
    elab.bind("t", DimVector::time());
    elab.bind("x", DimVector::DIMENSIONLESS);
    // sin of a length is meaningless; sin of a number is a number.
    assert!(elab.infer_dimension(&call("sin", vec![*var("l")])).is_err());
    assert!(elab.infer_dimension(&call("exp", vec![*var("t")])).is_err());
    assert!(
        elab.infer_dimension(&call("sin", vec![*var("x")]))
            .unwrap()
            .is_dimensionless()
    );
    // sqrt halves the dimension, abs keeps it, max needs agreeing arguments.
    let area = bin("*", var("l"), var("l"));
    assert_eq!(
        elab.infer_dimension(&call("sqrt", vec![area])).unwrap(),
        DimVector::length()
    );
    assert_eq!(
        elab.infer_dimension(&call("abs", vec![*var("l")])).unwrap(),
        DimVector::length()
    );
    assert!(
        elab.infer_dimension(&call("max", vec![*var("l"), *var("t")]))
            .is_err()
    );
    assert_eq!(
        elab.infer_dimension(&call("max", vec![*var("l"), *var("l")]))
            .unwrap(),
        DimVector::length()
    );
}
