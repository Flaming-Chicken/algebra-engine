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

#[test]
fn html_report_is_escaped_structured_and_printable() {
    use urae_notebook::export::to_html_report;
    let doc = "# Title <b>\n## Section\n// prose & more\n\nx^2 + 1\n2 + 3\n";
    let html = to_html_report(
        "My <report>",
        doc,
        &[None, None, None, None, None, Some("5".into())],
    );

    assert!(html.starts_with("<!DOCTYPE html>"));
    // Everything user-supplied is escaped: no raw tag from the document or the title survives.
    assert!(html.contains("<h1>Title &lt;b&gt;</h1>"), "{html}");
    assert!(html.contains("My &lt;report&gt;"));
    assert!(!html.contains("<b>") && !html.contains("<report>"));
    assert!(html.contains("<h2>Section</h2>"));
    assert!(html.contains("<p>prose &amp; more</p>"));
    // Formula blocks carry the result only where one was supplied (line index 5 is `2 + 3`).
    assert!(html.contains("<code>x^2 + 1</code></div>"));
    assert!(html.contains("<code>2 + 3</code><div class=\"result\">= 5</div>"));
    assert_eq!(html.matches("class=\"result\"").count(), 1);
    // Printable and inert: a print stylesheet, no script.
    assert!(html.contains("@media print"));
    assert!(!html.to_lowercase().contains("<script"));

    // A hostile line cannot inject markup.
    let evil = to_html_report("t", "<script>alert(1)</script>", &[]);
    assert!(!evil.contains("<script>alert"));
    assert!(evil.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));

    // An empty document is still a valid page, and short result slices are fine.
    let empty = to_html_report("empty", "", &[]);
    assert!(empty.contains("<body>") && empty.contains("</html>"));
}
