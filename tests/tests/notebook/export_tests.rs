//! The notebook exporters translate the parsed math; they do not paste the raw buffer.

use urae_notebook::export::{to_latex, to_lean, to_sympy};

const DOC: &str =
    "# Expansion\n(x + y)^2 = x^2 + 2*x*y + y^2\nsin(x)^2 + cos(x)^2\n\nthis is prose, not math (";

#[test]
fn latex_export_typesets_formulas_and_comments_out_the_rest() {
    let tex = to_latex(DOC);
    assert!(tex.starts_with(r"\documentclass{article}"), "{tex}");
    assert!(
        tex.contains(r"\usepackage{amsmath}") && tex.contains(r"\end{document}"),
        "{tex}"
    );
    assert!(tex.contains("% Expansion"), "{tex}");
    // The equation is typeset from the parse tree, not copied.
    assert!(
        tex.contains(r"\[ ") && tex.contains(" = ") && tex.contains(r"\operatorname{sin}"),
        "{tex}"
    );
    assert!(
        !tex.contains("2*x*y"),
        "raw text leaked into the output: {tex}"
    );
    assert!(tex.contains("% not typeset: this is prose"), "{tex}");
}

#[test]
fn sympy_export_declares_symbols_and_builds_expressions() {
    let py = to_sympy(DOC);
    assert!(py.contains("import sympy as sp"), "{py}");
    assert!(py.contains("sp.symbols("), "{py}");
    assert!(py.contains("sp.Eq(") && py.contains("**"), "{py}");
    assert!(py.contains("sp.sin(x)") && py.contains("sp.cos(x)"), "{py}");
    assert!(
        py.contains("# Expansion") && py.contains("# not translated: this is prose"),
        "{py}"
    );
    // No raw document pasted into a docstring.
    assert!(!py.contains("\"\"\""), "{py}");
}

#[test]
fn lean_export_states_equations_and_skips_everything_else() {
    let lean = to_lean(DOC);
    assert!(lean.contains("import Mathlib"), "{lean}");
    assert!(lean.contains("theorem step_1 (x y : ℝ)"), "{lean}");
    assert!(lean.contains("ring"), "{lean}");
    assert!(
        lean.contains("-- not an equation, skipped: sin(x)^2 + cos(x)^2"),
        "{lean}"
    );
    assert!(
        lean.contains("-- not an equation, skipped: this is prose"),
        "{lean}"
    );
}

#[test]
fn exports_of_an_empty_document_are_valid_skeletons() {
    assert!(to_latex("").contains(r"\begin{document}"));
    assert!(to_sympy("").contains("import sympy as sp"));
    assert!(to_lean("").contains("import Mathlib"));
}
