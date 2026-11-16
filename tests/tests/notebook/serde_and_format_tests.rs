use urae::prelude::*;
use urae_notebook::notebook::{NotebookSettings, SessionData, SymbolMetadata, SymbolRole};

#[test]
fn test_json_serde_session_and_metadata() {
    let mut session = SessionData::default();
    session.slider_values.insert("a".to_string(), 2.75);

    let mut meta = SymbolMetadata::new("a", 2.75, SymbolRole::Parameter);
    meta.unit_str = Some("m/s^2".to_string());
    session.symbol_metadata.insert("a".to_string(), meta);

    // Test JSON Serialization
    let json_str = serde_json::to_string_pretty(&session).expect("JSON serialization failed");
    assert!(json_str.contains("\"a\""));
    assert!(json_str.contains("2.75"));

    // Test JSON Deserialization
    let deserialized: SessionData =
        serde_json::from_str(&json_str).expect("JSON deserialization failed");
    assert_eq!(deserialized.slider_values.get("a"), Some(&2.75));
    assert_eq!(
        deserialized
            .symbol_metadata
            .get("a")
            .and_then(|m| m.unit_str.as_deref()),
        Some("m/s^2")
    );
}

#[test]
fn test_toml_serde_notebook_settings() {
    let settings = NotebookSettings::default();

    // Test TOML Serialization
    let toml_str = toml::to_string(&settings).expect("TOML serialization failed");
    assert!(toml_str.contains("font_size"));

    // Test TOML Deserialization
    let deserialized: NotebookSettings =
        toml::from_str(&toml_str).expect("TOML deserialization failed");
    assert_eq!(deserialized.font_size, settings.font_size);
    assert_eq!(deserialized.auto_sliders, settings.auto_sliders);
}

#[test]
fn test_latex_input_parsing_and_output_formatting() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let formatter = LatexFormatter;

    // LaTeX inputs
    let latex_input = "\\frac{x^{2} + 1}{2} + \\sqrt{x} + \\sin(x)";
    let expr_id = parser.parse(latex_input).expect("LaTeX parsing failed");

    // LaTeX output formatting
    let formatted_latex = formatter
        .format(&graph, expr_id)
        .expect("LaTeX formatting failed");
    assert!(formatted_latex.contains("sin"));
    assert!(formatted_latex.contains("sqrt"));
}

#[test]
fn test_unicode_input_parsing_and_output_formatting() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);
    let unicode_formatter = urae::format::UnicodeFormatter;

    // Unicode math inputs
    let unicode_input = "x² + 3 · x + 1 + √x";
    let expr_id = parser.parse(unicode_input).expect("Unicode parsing failed");

    // Unicode 2D/1D output formatting
    let formatted_unicode = unicode_formatter
        .format(&graph, expr_id)
        .expect("Unicode formatting failed");
    assert!(!formatted_unicode.is_empty());
}

#[test]
fn test_python_exponent_parsing_and_rust_code_emission() {
    let graph = ExprGraph::new();
    let parser = ExprParser::new(&graph);

    // Python exponent notation x**2 + 3*x
    let py_input = "x**2 + 3*x + sin(x)";
    let expr_id = parser
        .parse(py_input)
        .expect("Python exponent input parsing failed");

    // Code generation emission to Rust/C code string
    let emitted_code = urae::codegen::CodeGenerator::emit_code(
        &graph,
        expr_id,
        urae::codegen::TargetLanguage::Rust,
    )
    .unwrap();
    assert!(emitted_code.contains("x"));
}

#[test]
fn test_lean4_and_coq_proof_export() {
    use urae::proof::{ProofTrace, UnificationEngine};

    let graph = ExprGraph::new();
    let engine = UnificationEngine::new();

    let parser = ExprParser::new(&graph);
    let a = parser.parse("x + 0").unwrap();
    let b = parser.parse("x").unwrap();

    let _subst = engine.unify(&graph, a, b).expect("Unification failed");

    let mut trace = ProofTrace::new();
    trace.add_step("add_zero_identity", a, b);

    // Lean 4 export
    let lean4_script = trace.export_lean4();
    assert!(lean4_script.contains("Mathlib"));
    assert!(lean4_script.contains("add_zero_identity"));

    // Coq export
    let coq_script = trace.export_coq();
    assert!(coq_script.contains("Require Import"));
}

#[test]
fn test_compressed_bson_roundtrip() {
    use urae_notebook::notebook::{
        export_session_to_compressed_bson, import_session_from_compressed_bson,
    };

    let session = SessionData::default();
    let bytes = export_session_to_compressed_bson(&session).expect("BSON export failed");
    assert!(!bytes.is_empty());

    let imported = import_session_from_compressed_bson(&bytes).expect("BSON import failed");
    assert_eq!(session.raw_document_text, imported.raw_document_text);
    assert_eq!(session.slider_values.len(), imported.slider_values.len());
}

#[test]
fn test_serde_backward_compatibility() {
    let legacy_json = r#"{"raw_document_text":"x^2 + 1","slider_values":{}}"#;
    let imported: Result<SessionData, _> = serde_json::from_str(legacy_json);
    assert!(imported.is_ok());
    let session = imported.unwrap();
    assert_eq!(session.raw_document_text, "x^2 + 1");
}
