//! Integration Tests for Phase 17: GPU Compute, 3D Complex Visualizer & Ecosystem Tooling.

use algebra_engine::cad::Mesh3D;
use algebra_engine::gpu::WgslShaderGenerator;
use algebra_engine::html_export::StandaloneHtmlExporter;
use algebra_engine::visualizer::{DomainColoring, RiemannSurface, VectorFieldVisualizer};
use algebra_engine::{domain_color, streamline_2d, wgsl_grid};

#[test]
fn test_domain_coloring_hsv_mapping() {
    // 1. Check positive real number z = 1.0 (phase = 0 -> hue = 180)
    let c_pos = DomainColoring::color_at(1.0, 0.0);
    assert!(c_pos.r >= 0.0 && c_pos.r <= 1.0);
    assert!(c_pos.g >= 0.0 && c_pos.g <= 1.0);
    assert!(c_pos.b >= 0.0 && c_pos.b <= 1.0);
    assert_eq!(c_pos.a, 1.0);

    // 2. Check imaginary z = i
    let c_im = DomainColoring::color_at(0.0, 1.0);
    assert_eq!(c_im.a, 1.0);

    // 3. Grid evaluation for f(z) = z^2 - 1
    let grid = DomainColoring::evaluate_grid(-2.0, 2.0, -2.0, 2.0, 20, 20, |x, y| {
        // (x + iy)^2 - 1 = x^2 - y^2 - 1 + 2ixy
        (x * x - y * y - 1.0, 2.0 * x * y)
    });
    assert_eq!(grid.len(), 20 * 20);
}

#[test]
fn test_riemann_surface_sqrt_and_log_mesh_generation() {
    let sqrt_mesh = RiemannSurface::sqrt_surface(2.0, 8, 16);
    assert_eq!(sqrt_mesh.name, "RiemannSurface_Sqrt");
    assert_eq!(sqrt_mesh.vertices.len(), (8 + 1) * (16 + 1));
    assert_eq!(sqrt_mesh.triangles.len(), 8 * 16 * 2);
    for v in &sqrt_mesh.vertices {
        assert!(v.iter().all(|c| c.is_finite()));
        assert!(v[0] * v[0] + v[1] * v[1] <= 4.0 + 1e-9);
    }
    for tri in &sqrt_mesh.triangles {
        assert!(tri.iter().all(|&idx| idx < sqrt_mesh.vertices.len()));
    }

    let log_mesh = RiemannSurface::log_surface(2.0, 2, 8, 16);
    assert_eq!(log_mesh.name, "RiemannSurface_Log");
    assert_eq!(log_mesh.vertices.len(), (2 * 16 + 1) * (8 + 1));
    assert_eq!(log_mesh.triangles.len(), 2 * 16 * 8 * 2);
    assert!((log_mesh.vertices.first().unwrap()[2] - 0.0).abs() < 1e-12);
    assert!((log_mesh.vertices.last().unwrap()[2] - 2.0).abs() < 1e-12);
    for tri in &log_mesh.triangles {
        assert!(tri.iter().all(|&idx| idx < log_mesh.vertices.len()));
    }
}

#[test]
fn test_vector_field_streamlines_circular_vortex() {
    // Circular vortex vector field: dx/dt = -y, dy/dt = x
    let field = |x: f64, y: f64| [-y, x];
    let seed = [1.0, 0.0];
    let dt = 0.05;
    let steps = 30;

    let path = VectorFieldVisualizer::trace_streamline_2d(seed, field, dt, steps);
    assert_eq!(path.len(), steps + 1);

    // Verify distance from origin is conserved (r^2 = x^2 + y^2 approx 1.0)
    for pt in &path {
        let r2 = pt[0] * pt[0] + pt[1] * pt[1];
        assert!((r2 - 1.0).abs() < 0.1);
    }
}

#[test]
fn test_wgsl_shader_generator_kernels() {
    // 1. Grid evaluation compute shader
    let grid_shader = WgslShaderGenerator::generate_grid_evaluation_shader("x * x + 2.0 * x", 64);
    assert_eq!(grid_shader.kernel_name, "grid_evaluation");
    assert_eq!(grid_shader.workgroup_size, [64, 1, 1]);
    assert!(
        grid_shader
            .wgsl_source
            .contains("@compute @workgroup_size(64, 1, 1)")
    );
    assert!(
        grid_shader
            .wgsl_source
            .contains("output_values[idx] = val;")
    );

    // 2. Matrix-Free FEM shader
    let fem_shader = WgslShaderGenerator::generate_matrix_free_fem_shader(3, 64);
    assert_eq!(fem_shader.kernel_name, "matrix_free_fem");
    assert!(fem_shader.wgsl_source.contains("atomicAdd"));

    // 3. Complex fractal shader
    let fractal_shader = WgslShaderGenerator::generate_complex_fractal_shader(100, 8);
    assert_eq!(fractal_shader.kernel_name, "complex_fractal");
    assert_eq!(fractal_shader.workgroup_size, [8, 8, 1]);
    assert!(fractal_shader.wgsl_source.contains("pixel_buffer"));
}

#[test]
fn test_standalone_html_exporter() {
    let mesh = Mesh3D::cube(1.0, 1.0, 1.0);
    let html = StandaloneHtmlExporter::export_mesh_to_html(
        "Unit Cube Analysis",
        "Demonstrating interactive 3D visualization in single-file HTML",
        "V = L^3 = 1.0",
        &mesh,
    );

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Unit Cube Analysis"));
    assert!(html.contains("V = L^3 = 1.0"));
    assert!(html.contains("three.min.js"));
    assert!(html.contains("katex.min.js"));
}

#[test]
fn test_visualizer_and_gpu_dsl_macros() {
    let c = domain_color!(1.0, 0.0);
    assert_eq!(c.a, 1.0);

    let path = streamline_2d!([1.0, 0.0], |x, y| [-y, x], 0.05, 10);
    assert_eq!(path.len(), 11);

    let shader = wgsl_grid!("x * x");
    assert!(shader.wgsl_source.contains("x * x"));
}

#[test]
fn test_sqrt_surface_height_selects_a_component_of_w() {
    use algebra_engine::visualizer::{RiemannHeight, RiemannSurface};
    let (re, im, modulus) = (
        RiemannSurface::sqrt_surface_with(2.0, 6, 12, RiemannHeight::Real),
        RiemannSurface::sqrt_surface_with(2.0, 6, 12, RiemannHeight::Imaginary),
        RiemannSurface::sqrt_surface_with(2.0, 6, 12, RiemannHeight::Modulus),
    );
    // The default view is the real part.
    assert_eq!(
        RiemannSurface::sqrt_surface(2.0, 6, 12).vertices,
        re.vertices
    );
    assert_eq!(re.vertices.len(), im.vertices.len());
    let mut distinct = false;
    for ((a, b), m) in re.vertices.iter().zip(&im.vertices).zip(&modulus.vertices) {
        // Same (x, y) base point in all three views.
        assert_eq!((a[0], a[1]), (b[0], b[1]));
        // w^2 = z: Re(w)^2 - Im(w)^2 = x and 2 Re(w) Im(w) = y.
        assert!((a[2] * a[2] - b[2] * b[2] - a[0]).abs() < 1e-9);
        assert!((2.0 * a[2] * b[2] - a[1]).abs() < 1e-9);
        // |w| = sqrt(|z|).
        assert!((m[2] - a[0].hypot(a[1]).sqrt()).abs() < 1e-9);
        distinct |= (a[2] - b[2]).abs() > 1e-6;
    }
    assert!(distinct, "the real and imaginary views must differ");
}

#[test]
fn generated_shaders_pass_structural_validation_and_broken_ones_do_not() {
    use algebra_engine::gpu::WgslShaderGenerator;
    let grid = WgslShaderGenerator::generate_grid_evaluation_shader("x * x + 2.0 * x", 64);
    let fem = WgslShaderGenerator::generate_matrix_free_fem_shader(3, 64);
    let fractal = WgslShaderGenerator::generate_complex_fractal_shader(100, 8);
    for kernel in [&grid, &fem, &fractal] {
        assert!(
            kernel.validate().is_ok(),
            "{}: {:?}",
            kernel.kernel_name,
            kernel.validate()
        );
    }

    // An expression with an unbalanced parenthesis breaks the shader and is caught.
    let broken = WgslShaderGenerator::generate_grid_evaluation_shader("x * (x + 1.0", 64);
    let message = broken.validate().unwrap_err();
    assert!(
        message.contains("unbalanced") || message.contains("unclosed"),
        "{message}"
    );

    // A workgroup above the default WebGPU limit (256 invocations) is rejected.
    let big = WgslShaderGenerator::generate_grid_evaluation_shader("x", 1024);
    assert!(big.validate().unwrap_err().contains("limit"));

    // The descriptor must agree with the source, and bindings must be unique and sufficient.
    let mut mismatch = grid.clone();
    mismatch.workgroup_size = [32, 1, 1];
    assert!(mismatch.validate().unwrap_err().contains("disagrees"));
    let mut duplicate = grid.clone();
    duplicate.wgsl_source = duplicate
        .wgsl_source
        .replace("@group(0) @binding(1)", "@group(0) @binding(0)");
    assert!(duplicate.validate().unwrap_err().contains("twice"));
    let mut starved = grid.clone();
    starved.input_buffer_count = 9;
    assert!(
        starved
            .validate()
            .unwrap_err()
            .contains("bindings declared")
    );
    let mut no_entry = grid.clone();
    no_entry.wgsl_source = no_entry.wgsl_source.replace("fn main(", "fn other(");
    assert!(no_entry.validate().unwrap_err().contains("entry point"));
}

#[test]
fn checked_polynomial_arithmetic_reports_overflow_and_the_wrapping_one_wraps() {
    use algebra_advanced::gpu::SimdPolyVector;
    let a = SimdPolyVector::new(vec![1, 2, 3]);
    let b = SimdPolyVector::new(vec![4, 5]);
    assert_eq!(a.checked_add(&b).unwrap().coeffs, vec![5, 7, 3]);
    assert_eq!(a.checked_multiply(&b).unwrap().coeffs, vec![4, 13, 22, 15]);
    // Agrees with the parallel version when nothing overflows.
    assert_eq!(
        a.multiply_simd(&b).coeffs,
        a.checked_multiply(&b).unwrap().coeffs
    );

    let big = SimdPolyVector::new(vec![i64::MAX, 1]);
    assert!(big.checked_add(&big).is_none());
    assert!(
        big.checked_multiply(&SimdPolyVector::new(vec![2]))
            .is_none()
    );
    // The unchecked methods wrap (arithmetic mod 2^64), as documented.
    assert_eq!(
        big.add_simd(&big).coeffs[0],
        i64::MAX.wrapping_add(i64::MAX)
    );
}

#[test]
fn the_enabled_flag_selects_between_the_parallel_and_the_sequential_path() {
    use algebra_advanced::gpu::GpuComputeKernel;
    use algebra_core::ExprGraph;
    use algebra_engine::matrix::SymbolicMatrix;
    let graph = ExprGraph::new();
    let entries = |base: i64| -> Vec<_> { (0..9).map(|i| graph.integer(base + i)).collect() };
    let a = SymbolicMatrix::new(3, 3, entries(1)).unwrap();
    let b = SymbolicMatrix::new(3, 3, entries(2)).unwrap();

    let parallel = GpuComputeKernel { enabled: true }
        .multiply_matrix_blocks(&graph, &a, &b)
        .unwrap();
    let sequential = GpuComputeKernel { enabled: false }
        .multiply_matrix_blocks(&graph, &a, &b)
        .unwrap();
    // The two paths give the same product.
    assert_eq!(parallel.elements, sequential.elements);
    // And it is the real product: row 0 of A is (1, 2, 3), column 0 of B is (2, 5, 8): 2 + 10 + 24.
    assert_eq!(parallel.elements[0], graph.integer(36));
}
