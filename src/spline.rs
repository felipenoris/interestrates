
pub struct Spline<'a> {
    x: & 'a Vec<i32>,
    y: & 'a Vec<f64>,
    params: Vec<f64>,
}

impl<'a> Spline<'a> {

    pub fn spline_fit(
        x: & 'a Vec<i32>,
        y: & 'a Vec<f64>,
    ) -> Self {

        assert!(x.len() == y.len());
        assert!(x.len() >= 2);

        let n = x.len();

        let x_f64: Vec<f64> = x.iter()
            .map(|x| *x as f64)
            .collect();

        let H: Vec<_> = x_f64.windows(2)
            .map(|w| w[1] - w[0])
            .collect();

        let A = y;

        let mut Alpha: Vec<_> = (1..n-1)
            .map(|i| (3.0/H[i])*(A[i+1] - A[i]) - (3.0/H[i-1])*(A[i] - A[i-1]))
            .collect();

        Alpha.insert(0, 0.0);

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

        for i in (1..n).rev() {
            C[i] = Z[i] - Mu[i] * C[i+1];
            B[i] = (A[i+1] - A[i]) / H[i] - H[i] * (C[i+1] + 2.0*C[i]) / 3.0;
            D[i] = (C[i+1] - C[i]) / (3.0*H[i]);
        }

        let params = (0..n-1)
            .flat_map(|i| [A[i], B[i], C[i], D[i]])
            .collect();

        Spline { x, y, params }
    }

    pub fn spline_int(&self, x_out: i32) -> f64 {

        if x_out > *self.x.last().unwrap() {
            // extrapolation after last point

            let n = self.x.len();
            let x = (x_out - self.x[n-1]) as f64;
            let dxn = (self.x[n-1] - self.x[n-2]) as f64;
            let i = 4*(n-2);
            let b = self.params[i+1] + 2.0*self.params[i+2]*dxn + 3.0*self.params[i+3]*dxn*dxn;

            return self.y[n-1] + b*x;

        } else if x_out < *self.x.first().unwrap() {
            // extrapolation before first point
            return self.params[0] + self.params[1]*((x_out - self.x[0]) as f64);
        } else {
            // find polynomial
            let mut i = 0;

            while x_out > self.x[i+1] {
                i += 1;
            }

            let x = (x_out - self.x[i]) as f64;
            let a = self.params[4*i];
            let b = self.params[4*i + 1];
            let c = self.params[4*i + 2];
            let d = self.params[4*i + 3];
            return a + b*x + c*x*x + d*x*x*x;
        }
    }
}
