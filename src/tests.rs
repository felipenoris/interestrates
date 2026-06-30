use bdays::{HolidayCalendar, date::Date};
use crate::daycount::DayCount;
use crate::rate::{Compounding, Rate};
use crate::curve::{Curve, CurvePoints, CurveMethod};
use crate::assert_approx_eq;
use std::mem::size_of;

#[test]
fn print_copy_struct_sizes() {
    println!("Date = {} bytes", size_of::<Date>());
    println!("DayCount = {} bytes", size_of::<DayCount>());
    println!("Compounding = {} bytes", size_of::<Compounding>());
    println!("Rate = {} bytes", size_of::<Rate>());
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
        curve_b252_ec_lin.zero_rate(maturity_13_days).annual_rate(),
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
        curve_b252_ec_lin.zero_rate(maturity_11_days).annual_rate(),
        0.10,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(cal.advance_bdays(dt_curve, 11-4)).annual_rate(),
        0.05,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(cal.advance_bdays(dt_curve, 23+4)).annual_rate(),
        0.18,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(dt_curve.advance_days(30)).annual_rate(),
        0.1925,
    );

    assert_approx_eq(
        curve_b252_ec_lin.zero_rate(maturity_21_days).annual_rate(),
        0.195,
    );

    test_curve_at_curve_date(&curve_b252_ec_lin);
}

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
        assert_approx_eq(curve_ac365_simple_linear.zero_rate(result.maturity).annual_rate(), result.zero_rate);
        assert_approx_eq(curve_ac365_simple_linear.factor(result.maturity), result.factor);
        assert_approx_eq(curve_ac365_simple_linear.discount(result.maturity), result.discount);
    }

    test_curve_at_curve_date(&curve_ac365_simple_linear);
}

#[test]
fn test_flat_forward_interpolation() {

    let vert_x = vec![11, 15, 19, 23];
    let vert_y = vec![0.10, 0.15, 0.20, 0.19];

    let dt_curve = Date::from_ymd(2015,08,03).unwrap();

    let curve_ac360_cont_ff = CurvePoints::new(
        dt_curve,
        DayCount::Actual360,
        Compounding::Continuous,
        CurveMethod::FlatForwardInterpolation,
        vert_x.clone(),
        vert_y.clone(),
    ).unwrap();

    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(9)).annual_rate(), 0.05833333333333);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(11)).annual_rate(), 0.1);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(13)).annual_rate(), 0.128846153846152);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(15)).annual_rate(), 0.15);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(19)).annual_rate(), 0.2);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(23)).annual_rate(), 0.19);
    assert_approx_eq(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(30)).annual_rate(), 0.1789166666666680);
    assert!(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(16)).annual_rate() > 0.15);
    assert!(curve_ac360_cont_ff.zero_rate(dt_curve.advance_days(17)).annual_rate() < 0.20);
    assert_approx_eq(curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(11), dt_curve.advance_days(15)).annual_rate(), 0.2875);
    assert_approx_eq(curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(11), dt_curve.advance_days(13)).annual_rate(), 0.2875);
    assert_approx_eq(curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(19), dt_curve.advance_days(23)).annual_rate(), 0.1425);
    assert_approx_eq(curve_ac360_cont_ff.factor(dt_curve.advance_days(13)), 1.00466361875533);
    assert_approx_eq(curve_ac360_cont_ff.forward_factor(dt_curve.advance_days(19), dt_curve.advance_days(23)), 1.00158458746737);
    assert_approx_eq(curve_ac360_cont_ff.discount(dt_curve.advance_days(20)), 0.9891083592630893);

    assert_approx_eq(curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(19), dt_curve.advance_days(23)).annual_rate(), curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(50), dt_curve.advance_days(51)).annual_rate());
    assert_approx_eq(curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(19), dt_curve.advance_days(23)).annual_rate(), curve_ac360_cont_ff.forward_rate(dt_curve.advance_days(50), dt_curve.advance_days(100)).annual_rate());

    test_curve_at_curve_date(&curve_ac360_cont_ff);
}

fn test_curve_at_curve_date(curve: &dyn Curve) {

    let dt_curve = curve.asof();

    assert_approx_eq(
        curve.factor(dt_curve),
        1.0,
    );

    assert_approx_eq(
        curve.forward_factor(dt_curve, dt_curve),
        1.0,
    );

    assert_approx_eq(
        curve.discount(dt_curve),
        1.0,
    );

    assert_approx_eq(
        curve.forward_discount(dt_curve, dt_curve),
        1.0,
    );
}

#[test]
fn test_thirty360() {
    let dc = DayCount::Thirty360;

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2012, 2,28).unwrap()).value(), 0.16666666666666666);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2012, 2,29).unwrap()).value(), 0.16944444444444445);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2012, 3, 1).unwrap()).value(), 0.175);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2016, 2,28).unwrap()).value(), 4.166666666666667);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2016, 2,29).unwrap()).value(), 4.169444444444444);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2011,12,28).unwrap(), Date::from_ymd(2016, 3, 1).unwrap()).value(), 4.175);

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 2,28).unwrap(), Date::from_ymd(2012, 3,28).unwrap()).value(), 0.08333333333333333);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 2,29).unwrap(), Date::from_ymd(2012, 3,28).unwrap()).value(), 0.08055555555555556);

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 3, 1).unwrap(), Date::from_ymd(2012, 3,28).unwrap()).value(), 0.075);

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,29).unwrap(), Date::from_ymd(2013, 8,29).unwrap()).value(), 1.25);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,29).unwrap(), Date::from_ymd(2013, 8,30).unwrap()).value(), 1.2527777777777778);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,29).unwrap(), Date::from_ymd(2013, 8,31).unwrap()).value(), 1.2555555555555555);

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,30).unwrap(), Date::from_ymd(2013, 8,29).unwrap()).value(), 1.2472222222222222);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,30).unwrap(), Date::from_ymd(2013, 8,30).unwrap()).value(), 1.25);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,30).unwrap(), Date::from_ymd(2013, 8,31).unwrap()).value(), 1.25);

    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,31).unwrap(), Date::from_ymd(2013, 8,29).unwrap()).value(), 1.2472222222222222);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,31).unwrap(), Date::from_ymd(2013, 8,30).unwrap()).value(), 1.25);
    assert_approx_eq( dc.year_fraction(Date::from_ymd(2012, 5,31).unwrap(), Date::from_ymd(2013, 8,31).unwrap()).value(), 1.25);
}