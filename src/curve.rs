use bdays::date::Date;
use crate::curve::CurveMethod::{FlatForwardInterpolation, LinearInterpolation, StepFunction};
use crate::daycount::YearFraction;
use crate::rate::{Compounding, Rate};
use crate::daycount::DayCount::{self, BDays252};

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
pub enum CurveMethod {
    FlatForwardInterpolation,
    LinearInterpolation,
    StepFunction,
}

impl CurveMethod {

    fn is_interpolation_method(&self) -> bool {
        match self {
            FlatForwardInterpolation | LinearInterpolation | StepFunction => true,
        }
    }
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

        let yf_fwd = YearFraction::from_value(zr_yf_end.year_fraction().value() - zr_yf_start.year_fraction().value());

        assert!(yf_fwd.value() >= 0.0);

        Rate::from_factor(
            zr_yf_start.compounding(),
            zr_yf_end.factor() / zr_yf_start.factor(),
            yf_fwd,
        )
    }
}

pub struct CurvePoints<'a> {
    asof: Date,
    daycount: DayCount<'a>,
    compounding: Compounding,
    method: CurveMethod,
    dtm: Vec<i32>,
    zero_rates: Vec<f64>,
}

impl<'a> CurvePoints<'a> {

    /// zero_rate = 0.123 means 12.30%
    pub fn new(
        asof: Date,
        daycount: DayCount<'a>,
        compounding: Compounding,
        method: CurveMethod,
        dtm: Vec<i32>,
        zero_rates: Vec<f64>,
    ) -> Result<Self, Error> {

        if let BDays252(cal) = daycount {
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

        Ok(CurvePoints { asof, daycount, compounding, method, dtm, zero_rates })
    }

    /// panics if maturity occurs before asof date
    fn days_to_maturity(&self, maturity: Date) -> i32 {

        let result = self.daycount.days(
            self.asof,
            maturity,
        );

        assert!(result >= 0, "Maturity date {} should be greater than curve observation date {}", maturity, self.asof);

        result
    }

    fn year_fraction(&self, maturity: Date) -> YearFraction {
        self.daycount.year_fraction(self.asof, maturity)
    }

    fn vertex_zero_rate(&self, vertex_index: usize) -> Rate {
        let yf = self.daycount.year_fraction_given_days(self.dtm[vertex_index]);
        Rate::new(self.compounding, self.zero_rates[vertex_index], yf)
    }
}

impl<'a> Curve for CurvePoints<'a> {

    fn asof(&self) -> Date {
        self.asof
    }

    fn zero_rate(&self, maturity: Date) -> Rate {

        if self.method.is_interpolation_method() && self.dtm.len() == 1 {
            // If this curve has only 1 vertex, this will be a flat curve
            return Rate::new(
                self.compounding,
                *self.zero_rates.first().unwrap(),
                self.year_fraction(maturity),
            );
        }

        match self.method {
            CurveMethod::LinearInterpolation => {
                let x_out = self.days_to_maturity(maturity);
                let (index_a, index_b) = interpolation_points(&self.dtm, x_out);

                Rate::new(
                    self.compounding,
                    linear_interpolation(
                        self.dtm[index_a] as f64,
                        self.zero_rates[index_a],
                        self.dtm[index_b] as f64,
                        self.zero_rates[index_b],
                        x_out as f64,
                    ),
                    self.year_fraction(maturity),
                )
            },
            CurveMethod::StepFunction => {
                Rate::new(
                    self.compounding,
                    step_function_interpolation(
                        &self.dtm,
                        &self.zero_rates,
                        self.days_to_maturity(maturity),
                    ),
                    self.year_fraction(maturity),
                )
            },
            CurveMethod::FlatForwardInterpolation => {
                let x_out = self.days_to_maturity(maturity);
                let yf_x_out = self.daycount.year_fraction_given_days(x_out);
                let (index_a, index_b) = interpolation_points(&self.dtm, x_out);

                let rate_yf_a = self.vertex_zero_rate(index_a);
                let rate_yf_b = self.vertex_zero_rate(index_b);

                let ln_px = linear_interpolation(
                    rate_yf_a.year_fraction().value(),
                    rate_yf_a.discount().ln(),
                    rate_yf_b.year_fraction().value(),
                    rate_yf_b.discount().ln(),
                    yf_x_out.value(),
                );

                Rate::from_discount(
                    self.compounding,
                    ln_px.exp(),
                    yf_x_out,
                )
            }
        }
    }
}

fn step_function_interpolation(
        x: &Vec<i32>,
        y: &Vec<f64>,
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

    for x in 1..=14 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.10,
        );
    }

    for x in 15..=18 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.15,
        );
    }

    for x in 19..=22 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.20,
        );
    }

    for x in 23..=30 {
        assert_approx_eq(
            step_function_interpolation(&vert_x, &vert_y, x),
            0.19,
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
    x: &Vec<i32>,
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