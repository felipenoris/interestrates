use bdays::date::Date;
use bdays::HolidayCalendar;
use std::rc::Rc;
use std::fmt;
use std::ops::Sub;

#[derive(Clone)]
pub enum DayCount {
    Actual360,
    Actual365,
    Thirty360,
    BDays252(Rc<dyn HolidayCalendar>),
}

impl fmt::Debug for DayCount {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DayCount::Actual360 => "Actual360",
                DayCount::Actual365 => "Actual365",
                DayCount::Thirty360 => "Thirty360",
                DayCount::BDays252(_) => "BDays252",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct YearFraction {
    daycount: DayCount,
    dtm: i32,
    yf: f64,
}

impl YearFraction {

    pub fn from_days(
        daycount: DayCount,
        dtm: i32,
    ) -> Self {

        let yf = (dtm as f64) / daycount.days_per_year() as f64;

        YearFraction {
            daycount,
            dtm,
            yf,
        }
    }

    pub fn from_dates(
        daycount: DayCount,
        start: Date,
        end: Date,
    ) -> Self {
        let dtm = daycount.days(start, end);
        Self::from_days(daycount, dtm)
    }

    /// days to maturity
    pub fn dtm(&self) -> i32 {
        self.dtm
    }

    /// year fraction value
    pub fn yf(&self) -> f64 {
        self.yf
    }

    pub fn daycount(&self) -> &DayCount {
        &self.daycount
    }
}

impl<'a, 'b> Sub<&'b YearFraction> for &'a YearFraction {
    type Output = YearFraction;

    /// safety: propagates lhd daycount
    fn sub(self, rhs: &'b YearFraction) -> YearFraction {
        YearFraction::from_days(
            self.daycount().clone(),
            self.dtm() - rhs.dtm(),
        )
    }
}

impl DayCount {

    pub fn days(&self, start: Date, end: Date) -> i32 {
        match self {
            DayCount::Actual360 | DayCount::Actual365 => end - start,
            DayCount::Thirty360 => daycount_thirty360(start, end),
            DayCount::BDays252(cal) => cal.bdays(start, end),
        }
    }

    fn days_per_year(&self) -> i32 {
        match self {
            DayCount::Actual360 | DayCount::Thirty360 => 360,
            DayCount::Actual365 => 365,
            DayCount::BDays252(_) => 252,
        }
    }

    pub fn advance_days(&self, start: Date, count: i32) -> Date {
        match self {
            DayCount::BDays252(cal) => cal.advance_bdays(start, count),
            _ => start.advance_days(count),
        }
    }
}

fn daycount_thirty360(start: Date, end: Date) -> i32 {
    let (y2, m2, mut d2) = end.to_ymd();
    let (y1, m1, mut d1) = start.to_ymd();

    let y_diff = y2 - y1;
    let m_diff = m2 - m1;

    if d1 >= 30 {
        d1 = 30;

        if d2 >= 30 {
            d2 = 30;
        }
    }

    let d_diff = d2 - d1;

    360 * y_diff + 30 * m_diff + d_diff
}
