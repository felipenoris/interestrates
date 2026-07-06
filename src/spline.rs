
#[cfg(test)]
use crate::assert_approx_eq;

#[derive(Clone)]
pub struct Spline {
    x: Vec<i32>,
    y: Vec<f64>,
    params: Vec<f64>,
}

impl Spline {

    pub fn spline_fit(
        x: Vec<i32>,
        y: Vec<f64>,
    ) -> Self {

        let params = spline_fit(&x, &y);

        Spline {
            x,
            y,
            params,
        }
    }

    pub fn spline_int(
        &self,
        x_out: i32,
    ) -> f64 {
        spline_int(&self.x, &self.y, &self.params, x_out)
    }
}

fn spline_fit(
    x: &[i32],
    y: &[f64],
) -> Vec<f64> {

    assert!(x.len() == y.len());
    assert!(x.len() >= 2);

    let n = x.len();
    let x_f64: Vec<f64> = x.iter().map(|x| *x as f64).collect();
    let H: Vec<_> = x_f64.windows(2).map(|w| w[1] - w[0]).collect();
    let A = y;

    debug_assert!(x_f64.len() == x.len());
    debug_assert!(H.len() == (x.len()-1));

    let mut Alpha: Vec<_> = (1..n-1)
        .map(|i| (3.0 / H[i]) * (A[i + 1] - A[i]) - (3.0 / H[i - 1]) * (A[i] - A[i - 1]))
        .collect();

    Alpha.insert(0, 0.0);

    debug_assert!(Alpha.len() == H.len());

    let mut L = vec![1.0; n];
    let mut Mu = vec![0.0; n];
    let mut Z = vec![0.0; n];
    let mut B = vec![0.0; n];
    let mut C = vec![0.0; n];
    let mut D = vec![0.0; n];

    for i in 1..n-1 {
        L[i] = 2.0 * (x_f64[i+1] -  x_f64[i-1]) - H[i-1] * Mu[i-1];
        Mu[i] = H[i] / L[i];
        Z[i] = (Alpha[i] - H[i-1]*Z[i-1]) /  L[i];
    }

    for i in (0..n-1).rev() {
        C[i] = Z[i] - Mu[i] * C[i+1];
        B[i] = (A[i+1] - A[i]) / H[i] - H[i] * (C[i+1] + 2.0*C[i]) / 3.0;
        D[i] = (C[i+1] - C[i]) / (3.0*H[i]);
    }

    let params = (0..n-1).flat_map(|i| [A[i], B[i], C[i], D[i]]).collect();

    params
}

fn spline_int(
    x: &[i32],
    y: &[f64],
    params: &[f64],
    x_out: i32,
) -> f64 {

    if x_out > *x.last().unwrap() {
        // extrapolation after last point

        let n = x.len();
        let x_f64 = (x_out - x[n - 1]) as f64;
        let dxn = (x[n - 1] - x[n - 2]) as f64;
        let i = 4 * (n - 2);
        let b = params[i + 1]
            + 2.0 * params[i + 2] * dxn
            + 3.0 * params[i + 3] * dxn * dxn;

        return y[n - 1] + b * x_f64;

    } else if x_out < *x.first().unwrap() {
        // extrapolation before first point
        return params[0] + params[1] * ((x_out - x[0]) as f64);
    } else {
        // find polynomial
        let mut i = 0;

        while x_out > x[i + 1] {
            i += 1;
        }

        let x_f64 = (x_out - x[i]) as f64;
        let a = params[4 * i];
        let b = params[4 * i + 1];
        let c = params[4 * i + 2];
        let d = params[4 * i + 3];
        return a + b * x_f64 + c * x_f64 * x_f64 + d * x_f64 * x_f64 * x_f64;
    }
}

#[test]
fn test_spline() {
    let vert_x = [11, 15, 19, 23, 25];
    let vert_y = [0.10, 0.12, 0.20, 0.22, 0.2];

    let spline_params = spline_fit(&vert_x, &vert_y);

    let y: Vec<_> = (1..31).map(|i| spline_int(&vert_x, &vert_y, &spline_params, i)).collect();

    let y_benchmark = [
        0.09756098,
        0.09780488,
        0.09804878,
        0.09829268,
        0.09853659,
        0.09878049,
        0.09902439,
        0.09926829,
        0.09951220,
        0.09975610,
        0.10000000,
        0.10054116,
        0.10286585,
        0.10875762,
        0.12000000,
        0.13753049,
        0.15890244,
        0.18082317,
        0.20000000,
        0.21371189,
        0.22152439,
        0.22357470,
        0.22000000,
        0.21137195,
        0.20000000,
        0.18817073,
        0.17634146,
        0.16451220,
        0.15268293,
        0.14085366,
    ];

    assert_eq!(y.len(), y_benchmark.len());

    for i in 0..y.len() {
        assert_approx_eq(y[i], y_benchmark[i], 1.0e-8);
    }
}