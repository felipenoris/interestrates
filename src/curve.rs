use bdays::date::Date;
use bdays::HolidayCalendar;
use crate::daycount::YearFraction;
use crate::rate::{Compounding, Rate, RateYF};
use crate::daycount::DayCount::{self, BDays252};

use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidCurveDate{
        asof: Date,
    },

    Unsorted{
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
    LinearInterpolation,
    StepFunction,
}

pub trait Curve {

    fn zero_rate(&self, maturity: Date) -> RateYF;

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

    fn forward_rate(&self, fwd_date: Date, maturity: Date) -> RateYF {
        let zr_yf_start = self.zero_rate(fwd_date);
        let zr_yf_end = self.zero_rate(maturity);

        let yf_fwd = YearFraction::from_value(zr_yf_end.year_fraction().value() - zr_yf_start.year_fraction().value());

        assert!(yf_fwd.value() >= 0.0);

        RateYF::new(
            Rate::from_factor(
                zr_yf_start.rate().compounding(),
                zr_yf_end.factor() / zr_yf_start.factor(),
                yf_fwd,
            ),
            yf_fwd,
        )
    }
}

struct CurvePoints<'a> {
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

        if !dtm.is_sorted() {
            return Err(Error::Unsorted{dtm: dtm.clone()});
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
}

impl<'a> Curve for CurvePoints<'a> {

    fn zero_rate(&self, maturity: Date) -> RateYF {

        let result_as_f64: f64 = {
            if self.dtm.len() == 1 {
                // If this curve has only 1 vertice, this will be a flat curve
                *self.zero_rates.first().unwrap()
            } else {

                let x_out: i32 = self.days_to_maturity(maturity);

                match self.method {
                    CurveMethod::LinearInterpolation => {
                        let (index_a, index_b) = interpolation_points(&self.dtm, x_out);

                        linear_interpolation(
                            self.dtm[index_a] as f64,
                            self.zero_rates[index_a],
                            self.dtm[index_b] as f64,
                            self.zero_rates[index_b],
                            x_out as f64,
                        )
                    },
                    CurveMethod::StepFunction => {
                        step_function_interpolation(
                            &self.dtm,
                            &self.zero_rates,
                            x_out,
                        )
                    }
                }
            }
        };

        RateYF::new(
            Rate::new(self.compounding, result_as_f64),
            self.year_fraction(maturity),
        )
    }
}

fn step_function_interpolation(x: &Vec<i32>, y: &Vec<f64>, x_out: i32) -> f64 {
    if x_out <= *x.first().unwrap() {
        *y.first().unwrap()
    } else if x_out >= *x.last().unwrap() {
        *y.last().unwrap()
    } else {
        let pos = x.iter().rposition(|a| *a <= x_out).unwrap();
        y[pos]
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

#[test]
fn test_linear_method() {
    let vert_x = vec![11, 15, 19, 23];
    let vert_y = vec![0.10, 0.15, 0.20, 0.19];

    let dt_curve = Date::from_ymd(2015, 8, 3).unwrap();

    let cal = bdays::calendars::brazil::BRSettlement;

    let curve_b252_ec_lin = CurvePoints::new(
        dt_curve,
        DayCount::BDays252(&cal),
        Compounding::Exponential,
        CurveMethod::LinearInterpolation,
        vert_x.clone(),
        vert_y.clone(),
    ).unwrap();

    let maturity_11_days = cal.advance_bdays(dt_curve, 11);
    let maturity_13_days = cal.advance_bdays(dt_curve, 13);
    let maturity_14_days = cal.advance_bdays(dt_curve, 14);
    let maturity_21_days = cal.advance_bdays(dt_curve, 21);

    let yrs: f64 = (vert_x[0] as f64 + 2.0) / 252.0;
    let zero_rate_13_days: f64 = 0.125;
    let disc_13_days: f64 = 1.0 / ( (1.0 + zero_rate_13_days).powf(yrs) );

    assert_approx_eq(
        zero_rate_13_days,
        curve_b252_ec_lin.zero_rate(maturity_13_days).value(),
    );

    assert_approx_eq(
        disc_13_days,
        curve_b252_ec_lin.discount(maturity_13_days),
    );

    assert_approx_eq(
        curve_b252_ec_lin.discount(maturity_14_days) / curve_b252_ec_lin.discount(maturity_13_days),
        curve_b252_ec_lin.forward_discount(maturity_13_days, maturity_14_days),
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(maturity_11_days).value(),
        0.10,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(cal.advance_bdays(dt_curve, 11-4)).value(),
        0.05,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(cal.advance_bdays(dt_curve, 23+4)).value(),
        0.18,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(dt_curve.advance_days(30)).value(),
        0.1925,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(maturity_21_days).value(),
        0.195,
    );
}

#[cfg(test)]
struct ZeroRateResult {
    maturity: Date,
    zero_rate: f64,
    factor: f64,
    discount: f64,
}

#[test]
fn test_linear_actual365() {
    let dt_curve = Date::from_ymd(2015, 08, 07).unwrap();

    let vert_x = vec![11, 15, 19, 23];
    let vert_y = vec![0.10, 0.15, 0.20, 0.19];

    let curve_ac365_simple_linear = CurvePoints::new(
        dt_curve,
        DayCount::Actual365,
        Compounding::Simple,
        CurveMethod::LinearInterpolation,
        vert_x.clone(),
        vert_y.clone(),
    ).unwrap();

    let results = [
        ZeroRateResult{maturity: Date::from_ymd(2015,08,17).unwrap(), zero_rate: 0.0875, factor: 1.00239726027397, discount: 0.997608472839084},
        ZeroRateResult{maturity: Date::from_ymd(2015,08,18).unwrap(), zero_rate: 0.1, factor: 1.00301369863014, discount: 0.996995356459984},
        ZeroRateResult{maturity: Date::from_ymd(2015,08,19).unwrap(), zero_rate: 0.1125, factor: 1.00369863013699, discount: 0.996314999317592},
        ZeroRateResult{maturity: Date::from_ymd(2015,08,20).unwrap(), zero_rate: 0.1250, factor: 1.00445205479452, discount: 0.995567678145244},
        ZeroRateResult{maturity: Date::from_ymd(2015,08,21).unwrap(), zero_rate: 0.1375, factor: 1.00527397260274, discount: 0.994753696259454},
        ZeroRateResult{maturity: Date::from_ymd(2015,08,22).unwrap(), zero_rate: 0.15, factor: 1.00616438356164, discount: 0.993873383253914},
    ];

    for result in &results {
        assert_approx_eq(curve_ac365_simple_linear.zero_rate(result.maturity).value(), result.zero_rate);
        assert_approx_eq(curve_ac365_simple_linear.factor(result.maturity), result.factor);
        assert_approx_eq(curve_ac365_simple_linear.discount(result.maturity), result.discount);
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