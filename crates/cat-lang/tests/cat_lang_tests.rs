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

    let dim = elab.infer_dimension(&expr).expect("Dimension inference must succeed");
    assert_eq!(dim, DimVector::velocity());

    // Inconsistent addition: length + time => Error
    let bad_add = CatExpr::BinaryOp {
        op: "+".into(),
        lhs: Box::new(CatExpr::Variable("length".into())),
        rhs: Box::new(CatExpr::Variable("time".into())),
    };
    let err = elab.infer_dimension(&bad_add);
    assert!(err.is_err(), "Adding length and time must fail dimensional elaboration");
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
    let restored: CatScriptAst = CatBinary::load_script(&bytes).expect("Deserialization from .catb must succeed");
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
