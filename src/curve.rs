use bdays::date::Date;
use crate::daycount::YearFraction;
use crate::rate::{Compounding, Rate};
use crate::daycount::DayCount::{self, BDays252};
use crate::spline::Spline;

use std::error;
use std::fmt;

#[cfg(test)]
use crate::assert_approx_eq;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidCurveDate{
        asof: Date,
    },

    UnsortedOrDuplicates{
        dtm: Vec<i32>
    },

    BadSize{
        dtm: Vec<i32>,
        zero_rates: Vec<f64>,
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

impl error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationMethod {
    FlatForwardInterpolation,
    LinearInterpolation,
    StepFunction,
    CubicSplineOnRates,
    //CubicSplineOnDiscountFactors,
}

impl InterpolationMethod {

    fn is_cubic_spline_on_rates(&self) -> bool {
        match self {
            InterpolationMethod::CubicSplineOnRates => true,
            _ => false,
        }
    }

    //fn is_cubic_spline_on_discount_factors(&self) -> bool {
    //    match self {
    //        InterpolationMethod::CubicSplineOnDiscountFactors => true,
    //        _ => false,
    //    }
    //}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParametricMethod {
    NelsonSiegel,
    Svensson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveMethod {
    Interpolation{
        before_first: InterpolationMethod,
        inner: InterpolationMethod,
        after_last: InterpolationMethod,
    },

    Parametric(ParametricMethod),
}

impl CurveMethod {

    fn is_interpolation_method(&self) -> bool {
        match self {
            CurveMethod::Parametric(_) => false,
            CurveMethod::Interpolation{ .. } => true,
        }
    }

    fn needs_spline_on_rates(&self) -> bool {
        match self {
            CurveMethod::Parametric(_) => false,
            CurveMethod::Interpolation { before_first, inner, after_last } => {
                before_first.is_cubic_spline_on_rates()
                || inner.is_cubic_spline_on_rates()
                || after_last.is_cubic_spline_on_rates()
            }
        }
    }

    //fn needs_spline_on_discount_factors(&self) -> bool {
    //    match self {
    //        CurveMethod::Parametric(_) => false,
    //        CurveMethod::Interpolation { before_first, inner, after_last } => {
    //            before_first.is_cubic_spline_on_discount_factors()
    //            || inner.is_cubic_spline_on_discount_factors()
    //            || after_last.is_cubic_spline_on_discount_factors()
    //        }
    //    }
    //}
}

pub trait Curve {

    fn asof(&self) -> Date;

    fn zero_rate(&self, maturity: Date) -> Rate;

    fn factor(&self, maturity: Date) -> f64 {
        self.zero_rate(maturity).factor()
    }

    fn discount(&self, maturity: Date) -> f64 {
        self.zero_rate(maturity).discount()
    }

    fn forward_factor(&self, fwd_date: Date, maturity: Date) -> f64 {
        self.factor(maturity) / self.factor(fwd_date)
    }

    fn forward_discount(&self, fwd_date: Date, maturity: Date) -> f64 {
        1.0 / self.forward_factor(fwd_date, maturity)
    }

    fn forward_rate(&self, fwd_date: Date, maturity: Date) -> Rate {
        let zr_yf_start = self.zero_rate(fwd_date);
        let zr_yf_end = self.zero_rate(maturity);

        let yf_fwd = zr_yf_end.year_fraction() - zr_yf_start.year_fraction();

        assert!(yf_fwd.yf() >= 0.0);

        Rate::from_factor(
            zr_yf_start.compounding(),
            zr_yf_end.factor() / zr_yf_start.factor(),
            yf_fwd,
        )
    }
}

#[derive(Clone)]
pub struct CurvePoints {
    asof: Date,
    daycount: DayCount,
    compounding: Compounding,
    method: CurveMethod,
    dtm: Vec<i32>,
    zero_rates: Vec<f64>,
    spline_fit_on_rates: Option<Spline>,
    //spline_fit_on_discount_factors: Option<Spline>,
    parametric_params: Option<Vec<f64>>,
}

impl CurvePoints {

    /// Builds instance of interpolated curve
    /// zero_rate = 0.123 means 12.30%
    pub fn from_points(
        asof: Date,
        daycount: DayCount,
        compounding: Compounding,
        method: CurveMethod,
        dtm: Vec<i32>,
        zero_rates: Vec<f64>,
    ) -> Result<Self, Error> {

        if let BDays252(cal) = &daycount {
            if !cal.is_bday(asof) {
                return Err(Error::InvalidCurveDate{asof});
            }
        }

        if !dtm.is_sorted() || !dtm.windows(2).all(|w| w[0] != w[1]) {
            return Err(Error::UnsortedOrDuplicates{dtm: dtm.clone()});
        }

        if dtm.is_empty() || dtm.len() != zero_rates.len() {
            return Err(Error::BadSize{dtm, zero_rates});
        }

        let spline_fit_on_rates = {
            if method.needs_spline_on_rates() {
                Option::Some(Spline::spline_fit(dtm.clone(), zero_rates.clone()))
            } else {
                Option::None
            }
        };

        //let spline_fit_on_discount_factors = {
        //    if method.needs_spline_on_discount_factors() {
        //        Option::Some(Spline::spline_fit(dtm.clone(), zero_rates.clone()))
        //    } else {
        //        Option::None
        //    }
        //}

        Ok(CurvePoints {
            asof,
            daycount,
            compounding,
            method,
            dtm,
            zero_rates,
            spline_fit_on_rates,
            parametric_params: Option::None,
        })
    }

    /// Builds instance of parametric curve
    pub fn from_parameters(
        asof: Date,
        daycount: DayCount,
        compounding: Compounding,
        method: CurveMethod,
        parameters: Vec<f64>,
    ) -> Result<Self, Error> {

        if let BDays252(cal) = &daycount {
            if !cal.is_bday(asof) {
                return Err(Error::InvalidCurveDate{asof});
            }
        }

        Ok(CurvePoints {
            asof,
            daycount,
            compounding,
            method,
            dtm: Vec::<i32>::new(),
            zero_rates: Vec::<f64>::new(),
            spline_fit_on_rates: Option::None,
            parametric_params: Option::Some(parameters),
        })
    }

    fn year_fraction(&self, maturity: Date) -> YearFraction {
        YearFraction::from_dates(
            self.daycount.clone(),
            self.asof,
            maturity,
        )
    }

    fn vertex_zero_rate(&self, vertex_index: usize) -> Rate {
        let yf = YearFraction::from_days(self.daycount.clone(), self.dtm[vertex_index]);
        Rate::from_annual_rate(self.compounding, self.zero_rates[vertex_index], yf)
    }
}

fn zero_rate_interpolation(
        interp_method: InterpolationMethod,
        curve: &CurvePoints,
        yf: YearFraction,
    ) -> Rate {

    match interp_method {
        InterpolationMethod::LinearInterpolation => {

            let (index_a, index_b) = interpolation_points(&curve.dtm, yf.dtm());

            Rate::from_annual_rate(
                curve.compounding,
                linear_interpolation(
                    curve.dtm[index_a] as f64,
                    curve.zero_rates[index_a],
                    curve.dtm[index_b] as f64,
                    curve.zero_rates[index_b],
                    yf.dtm() as f64,
                ),
                yf,
            )
        },
        InterpolationMethod::StepFunction => {

            Rate::from_annual_rate(
                curve.compounding,
                step_function_interpolation(
                    &curve.dtm,
                    &curve.zero_rates,
                    yf.dtm(),
                ),
                yf,
            )
        },
        InterpolationMethod::FlatForwardInterpolation => {

            let (index_a, index_b) = interpolation_points(&curve.dtm, yf.dtm());

            let rate_yf_a = curve.vertex_zero_rate(index_a);
            let rate_yf_b = curve.vertex_zero_rate(index_b);

            let ln_px = linear_interpolation(
                rate_yf_a.year_fraction().yf(),
                rate_yf_a.discount().ln(),
                rate_yf_b.year_fraction().yf(),
                rate_yf_b.discount().ln(),
                yf.yf(),
            );

            Rate::from_discount(
                curve.compounding,
                ln_px.exp(),
                yf,
            )
        },
        InterpolationMethod::CubicSplineOnRates => {
            Rate::from_annual_rate(
                curve.compounding,
                curve.spline_fit_on_rates.as_ref().unwrap().spline_int(yf.dtm()),
                yf,
            )
        }
    }
}

impl Curve for CurvePoints {

    fn asof(&self) -> Date {
        self.asof
    }

    fn zero_rate(&self, maturity: Date) -> Rate {

        if self.method.is_interpolation_method() && self.dtm.len() == 1 {
            // If this curve has only 1 vertex, this will be a flat curve
            return Rate::from_annual_rate(
                self.compounding,
                *self.zero_rates.first().unwrap(),
                self.year_fraction(maturity),
            );
        }

        match self.method {
            CurveMethod::Interpolation{before_first, inner, after_last} => {
                let yf = self.year_fraction(maturity);

                if yf.dtm() < *self.dtm.first().unwrap() {
                    zero_rate_interpolation(before_first, self, yf)
                } else if yf.dtm() > *self.dtm.last().unwrap() {
                    zero_rate_interpolation(after_last, self, yf)
                } else {
                    zero_rate_interpolation(inner, self, yf)
                }
            },
            CurveMethod::Parametric(parametric_method) => {

                let yf_maturity = self.year_fraction(maturity);

                match parametric_method {
                    ParametricMethod::NelsonSiegel => {
                        nelson_siegel(
                            self.parametric_params.as_ref().unwrap(),
                            self.compounding,
                            yf_maturity,
                        )
                    },
                    ParametricMethod::Svensson => {
                        svensson(
                            self.parametric_params.as_ref().unwrap(),
                            self.compounding,
                            yf_maturity,
                        )
                    },
                }
            }
        }
    }
}

fn step_function_interpolation(
        x: &[i32],
        y: &[f64],
        x_out: i32,
    ) -> f64 {

    if x_out <= *x.first().unwrap() {
        *y.first().unwrap()
    } else if x_out >= *x.last().unwrap() {
        *y.last().unwrap()
    } else {
        let pos = x.iter().rposition(|a| *a <= x_out).unwrap();
        y[pos]
    }
}

#[test]
fn test_step_function_interpolation() {
    let vert_x = vec![11, 15, 19, 23];
    let vert_y = vec![0.10, 0.15, 0.20, 0.19];
    let tol: f64 = 1e-12;

    for x in 1..=14 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.10,
            tol,
        );
    }

    for x in 15..=18 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.15,
            tol,
        );
    }

    for x in 19..=22 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.20,
            tol,
        );
    }

    for x in 23..=30 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.19,
            tol,
        );
    }
}

fn linear_interpolation(
    xa: f64,
    ya: f64,
    xb: f64,
    yb: f64,
    x_out: f64,
) -> f64 {
    (x_out - xa) * (yb - ya) / (xb - xa) + ya
}

/// Returns tuple (index_a, index_b) for input vector x
/// for interpolands on linear interpolation on point x_out
/// 
/// x should be sorted and should contain at least 2 distinct points,
/// or this function will panic.
fn interpolation_points(
    x: &[i32],
    x_out: i32,
) -> (usize, usize) {

    let index_a: usize;
    let index_b: usize;

    if x.len() < 2 {
        panic!("interpolation requires at least 2 points.");
    }

    if x_out <= *x.first().unwrap() {
        // Interpolation point is before first point
        // Slope will be determined by the 1st and 2nd points
        index_a = 0;
        index_b = 1;
    } else if x_out >= *x.last().unwrap() {
        // Interpolation point is after the last point
        // Slope will be determined by the last and last-1 points.
        index_b = x.len() - 1;
        index_a = index_b - 1;
    } else {
        // inner point
        index_a = x.iter().rposition(|a| *a < x_out).unwrap(); // last element before x_out on x
        index_b = index_a + 1; // first element after x_out on x

        // sanity-check
        assert!(x[index_a] < x_out && x_out <= x[index_b]);
    }

    // sanity-check
    assert!(x[index_a] < x[index_b]);

    (index_a, index_b)
}

fn nelson_siegel(
    params: &Vec<f64>,
    compounding: Compounding,
    yf_maturity: YearFraction,
) -> Rate {

    // beta1 = param[0]
    // beta2 = param[1]
    // beta3 = param[2]
    // lambda = param[3]

    let t = yf_maturity.yf();
    let exp_lambda_t = (-params[3]*t).exp();
    let f_beta2 = (1.0 - exp_lambda_t) / (params[3]*t);

    let annual_rate = params[0] + params[1]*f_beta2 + params[2]*(f_beta2 - exp_lambda_t);

    Rate::from_annual_rate(
        compounding,
        annual_rate,
        yf_maturity,
    )
}

fn svensson(
    params: &Vec<f64>,
    compounding: Compounding,
    yf_maturity: YearFraction,
) -> Rate {
    // beta1 = param[0]
    // beta2 = param[1]
    // beta3 = param[2]
    // beta4 = param[3]
    // lambda1 = param[4]
    // lambda2 = param[5]

    let t = yf_maturity.yf();
    let exp_lambda1_t = (-params[4]*t).exp();
    let exp_lambda2_t = (-params[5]*t).exp();
    let f_beta2 = (1.0 - exp_lambda1_t) / (params[4]*t);

    let annual_rate = params[0] + params[1]*f_beta2 + params[2]*(f_beta2 - exp_lambda1_t) +
            params[3]*( (1.0 - exp_lambda2_t)/(params[5]*t) - exp_lambda2_t);

    Rate::from_annual_rate(
        compounding,
        annual_rate,
        yf_maturity,
    )   
}