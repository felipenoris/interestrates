pub mod spline;
pub mod daycount;
pub mod rate;
pub mod curve;

#[cfg(test)]
mod tests;

#[cfg(test)]
fn assert_approx_eq(a: f64, b: f64) {

    let tol: f64 = 1e-12;

    assert!(
        (a - b).abs() < tol,
        "left={} right={} diff={} tol={}",
        a,
        b,
        (a - b).abs(),
        tol,
    )
}
