use super::util::*;

#[test]
fn transform() {
    fn assert_affine_close(actual: Affine, expected: Affine) {
        let a = actual.as_coeffs();
        let e = expected.as_coeffs();
        for i in 0..6 {
            let diff = (a[i] - e[i]).abs();
            assert!(diff <= 1e-9, "affine coeff[{}] differs too much: actual={} expected={} diff={}", i, a[i], e[i], diff);
        }
    }

    // None
    let styles = apply_css_to_tree(".root { transform: none; }", single_node_tree);
    assert_eq!(styles[0].transform, Affine::IDENTITY);

    // Normal use (CSS matrix(a, b, c, d, e, f))
    let styles = apply_css_to_tree(".root { transform: matrix(1, 0, 0, 1, 10, 20); }", single_node_tree);
    assert_eq!(styles[0].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));

    // translate(tx, ty)
    let styles = apply_css_to_tree(".root { transform: translate(10px, 20px); }", single_node_tree);
    assert_eq!(styles[0].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));

    // scale(sx, sy)
    let styles = apply_css_to_tree(".root { transform: scale(2, 3); }", single_node_tree);
    assert_eq!(styles[0].transform, Affine::new([2.0, 0.0, 0.0, 3.0, 0.0, 0.0]));

    // rotate(angle)
    // (use approx because sin/cos are not exactly representable for most angles)
    let styles = apply_css_to_tree(".root { transform: rotate(90deg); }", single_node_tree);
    let theta = std::f64::consts::PI / 2.0;
    let expected = Affine::new([theta.cos(), theta.sin(), -theta.sin(), theta.cos(), 0.0, 0.0]);
    assert_affine_close(styles[0].transform, expected);

    // skew(ax, ay) — test ay = 0 so expected is the skewX(ax) matrix
    let styles = apply_css_to_tree(".root { transform: skew(45deg, 0deg); }", single_node_tree);
    let ax = std::f64::consts::PI / 4.0;
    let expected = Affine::new([1.0, 0.0, ax.tan(), 1.0, 0.0, 0.0]);
    assert_affine_close(styles[0].transform, expected);

    // Initial
    let styles = apply_css_to_tree(".child { transform: matrix(1, 0, 0, 1, 10, 20); } .right { transform: initial; }", two_child_tree);
    assert_eq!(styles[1].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));
    assert_eq!(styles[2].transform, Affine::IDENTITY);

    // Inherit
    let styles = apply_css_to_tree(".parent { transform: matrix(1, 0, 0, 1, 10, 20); } .child { transform: inherit; }", one_child_tree);
    assert_eq!(styles[0].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));
    assert_eq!(styles[1].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { transform: matrix(1, 0, 0, 1, 10, 20); }", one_child_tree);
    assert_eq!(styles[0].transform, Affine::new([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]));
    assert_eq!(styles[1].transform, Affine::IDENTITY);
}

#[test]
fn transform_reject() {
    fn reject(value: &str) {
        let css = format!(".root {{ transform: {}; }}", value);
        let styles = apply_css_to_tree(&css, single_node_tree);
        assert_eq!(styles[0].transform, Style::default().transform, "expected parser to reject transform: {}", value);
    }

    for value in [
        // Not a function / wrong tokens
        "auto",                            // unsupported keyword
        "foo",                             // unknown identifier
        "#fff",                            // not a transform value
        "10px",                            // not a transform function
        "rotate",                          // missing function call
        "matrix",                          // missing function call
        "translate",                       // missing function call
        "scale",                           // missing function call
        "skew",                            // missing function call
        "none rotate(10deg)",              // trailing token after keyword
        "matrix(1,0,0,1,10,20) extra",     // trailing token
        "matrix(1,0,0,1,10,20)!important", // contains !important
        // Parentheses / punctuation issues
        "matrix(1, 0, 0, 1, 10, 20",   // missing ')'
        "matrix(1, 0, 0, 1, 10, 20))", // extra ')'
        "matrix((1, 0, 0, 1, 10, 20)", // unexpected '('
        "matrix[1, 0, 0, 1, 10, 20]",  // wrong bracket type
        "matrix(1, 0, 0, 1, 10, 20),", // trailing comma after function
        // matrix() arity / separators / number parsing
        "matrix()",                       // empty args
        "matrix(1)",                      // wrong arity
        "matrix(1, 0, 0, 1, 10)",         // wrong arity
        "matrix(1, 0, 0, 1, 10, 20, 30)", // wrong arity
        "matrix(1, 0, 0, 1, 10,, 20)",    // empty arg
        "matrix(,1, 0, 0, 1, 10, 20)",    // leading comma
        "matrix(1, 0, 0, 1, 10, 20,)",    // trailing comma in arg list
        "matrix(1, 0, 0, 1, , 20)",       // empty arg
        "matrix(1, 0, 0, 1, 10, )",       // empty last arg
        "matrix(1, 0, 0, 1, 10px, 20)",   // units not allowed in coeffs
        "matrix(1, 0, 0, 1, 10, 20px)",   // units not allowed in coeffs
        "matrix(1, 0, 0, 1, 10%, 20)",    // percent not allowed in coeffs
        "matrix(1, 0, 0, 1, NaN, 20)",    // non-finite number
        "matrix(1, 0, 0, 1, inf, 20)",    // non-finite number
        "matrix(1, 0, 0, 1, -inf, 20)",   // non-finite number
        "matrix(a, 0, 0, 1, 10, 20)",     // non-number token
        // translate() malformed
        "translate()",                 // empty args
        "translate(10)",               // unitless length
        "translate(10, 20)",           // unitless lengths
        "translate(10px,)",            // trailing comma
        "translate(, 10px)",           // leading comma
        "translate(10px, 20px, 30px)", // too many args
        "translate(10%, 20px)",        // percent length not allowed here
        "translate(10px, 20%)",        // percent length not allowed here
        "translate(foo, 20px)",        // invalid token
        "translate(10px, bar)",        // invalid token
        "translate(-)",                // invalid numeric token
        // scale() malformed
        "scale()",        // empty args
        "scale(,)",       // missing numbers
        "scale(2,)",      // missing second arg
        "scale(,2)",      // missing first arg
        "scale(2, 3, 4)", // too many args
        "scale(2px, 3)",  // units not allowed
        "scale(2, 3px)",  // units not allowed
        "scale(2%)",      // percent not allowed
        "scale(foo)",     // invalid token
        // rotate() malformed
        "rotate()",            // empty args
        "rotate(90)",          // unitless angle
        "rotate(90px)",        // wrong unit
        "rotate(90deg, 1)",    // too many args
        "rotate(,90deg)",      // leading comma
        "rotate(90deg,)",      // trailing comma
        "rotate(foo)",         // invalid token
        "rotate(10% )",        // percent is not an angle
        "rotate(10deg 20deg)", // wrong separator / too many args
        // skew() malformed
        "skew()",                    // empty args
        "skew(10)",                  // unitless angle
        "skew(10px)",                // wrong unit
        "skew(10deg,)",              // missing second arg
        "skew(,10deg)",              // missing first arg
        "skew(10deg, 20deg, 30deg)", // too many args
        "skew(foo, 10deg)",          // invalid token
        "skew(10deg, bar)",          // invalid token
    ] {
        reject(value);
    }
}
