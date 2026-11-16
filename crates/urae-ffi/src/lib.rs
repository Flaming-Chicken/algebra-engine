//! # `urae-ffi`
//!
//! C ABI Foreign Function Interface (FFI) bindings for URAE.
//!
//! Allows embedding the Universal Rust Algebra Engine into C, C++, Python, WebAssembly (WASM),
//! and other native host environments.

#![allow(unsafe_code, unsafe_op_in_unsafe_fn)]

use algebra_core::ExprGraph;
use algebra_core::format::{Formatter, LatexFormatter};
use algebra_core::parser::ExprParser;
use algebra_engine::calculus::SymbolicCalculus;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Create a new `ExprGraph` instance on the heap and return an opaque pointer.
///
/// # Safety
/// Caller must free returned pointer using `urae_graph_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_graph_create() -> *mut ExprGraph {
    Box::into_raw(Box::new(ExprGraph::new()))
}

/// Free an `ExprGraph` instance created by `urae_graph_create`.
///
/// # Safety
/// `graph` must be a valid pointer created by `urae_graph_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_graph_free(graph: *mut ExprGraph) {
    if !graph.is_null() {
        drop(Box::from_raw(graph));
    }
}

/// Free a C-string allocated by URAE FFI functions.
///
/// # Safety
/// `s` must be a valid pointer allocated by URAE C ABI functions or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Parse expression string and return rendered LaTeX output string.
///
/// # Safety
/// `graph` and `input_str` must be valid, null-terminated pointers.
/// Returned string pointer must be freed using `urae_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_parse_and_format_latex(
    graph: *mut ExprGraph,
    input_str: *const c_char,
) -> *mut c_char {
    if graph.is_null() || input_str.is_null() {
        return std::ptr::null_mut();
    }

    let graph_ref = &*graph;
    let input_cstr = match CStr::from_ptr(input_str).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let parser = ExprParser::new(graph_ref);
    let formatter = LatexFormatter;

    match parser.parse(input_cstr) {
        Ok(expr_id) => match formatter.format(graph_ref, expr_id) {
            Ok(latex) => match CString::new(latex) {
                Ok(c_res) => c_res.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Parse expression, differentiate with respect to `var_str`, and return rendered LaTeX string.
///
/// # Safety
/// `graph`, `input_str`, and `var_str` must be valid pointers.
/// Returned pointer must be freed using `urae_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_differentiate_and_format_latex(
    graph: *mut ExprGraph,
    input_str: *const c_char,
    var_str: *const c_char,
) -> *mut c_char {
    if graph.is_null() || input_str.is_null() || var_str.is_null() {
        return std::ptr::null_mut();
    }

    let graph_ref = &*graph;
    let input = match CStr::from_ptr(input_str).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let var = match CStr::from_ptr(var_str).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let parser = ExprParser::new(graph_ref);
    let formatter = LatexFormatter;

    match parser.parse(input) {
        Ok(expr_id) => {
            let wrt_sym = graph_ref.symbols.get_or_intern(var);
            let diff_id = graph_ref.diff(expr_id, wrt_sym);
            let simp_id = algebra_engine::simplify::Simplifier::simplify(graph_ref, diff_id)
                .unwrap_or(diff_id);
            match formatter.format(graph_ref, simp_id) {
                Ok(latex) => match CString::new(latex) {
                    Ok(c_res) => c_res.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

use algebra_engine::session::UraeSession;

/// Create a new `UraeSession` instance on the heap and return an opaque pointer.
///
/// # Safety
/// Caller must free returned pointer using `urae_session_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_session_create() -> *mut UraeSession {
    Box::into_raw(Box::new(UraeSession::new()))
}

/// Free a `UraeSession` instance created by `urae_session_create`.
///
/// # Safety
/// `session` must be a valid pointer created by `urae_session_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_session_free(session: *mut UraeSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Execute a line of input against `UraeSession` and return rendered Unicode output string.
///
/// # Safety
/// `session` and `input_str` must be valid, non-null pointers.
/// Returned string pointer must be freed using `urae_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn urae_session_execute(
    session: *mut UraeSession,
    input_str: *const c_char,
) -> *mut c_char {
    if session.is_null() || input_str.is_null() {
        return std::ptr::null_mut();
    }

    let session_ref = &mut *session;
    let input = match CStr::from_ptr(input_str).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let res = session_ref.execute_line(input);
    let output = if res.is_error {
        res.error_msg.unwrap_or(res.output_text)
    } else {
        res.output_unicode
    };

    match CString::new(output) {
        Ok(c_res) => c_res.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
