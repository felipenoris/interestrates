use bdays::date::Date;
use bdays::HolidayCalendar;

pub enum DayCount<'a, H: HolidayCalendar> {
    Actual360,
    Actual365,
    Thirty360,
    BDays252(& 'a H), // BDays252(& 'a dyn HolidayCalendar),
}

#[derive(Debug, Clone, Copy)]
pub struct YearFraction {
    val: f64
}

impl YearFraction {

    pub fn value(&self) -> f64 {
        self.val
    }
}

impl<'a, H: HolidayCalendar> DayCount<'a, H> {

    pub fn day_count(&self, start: Date, end: Date) -> i32 {
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

    pub fn year_fraction(&self, start: Date, end: Date) -> YearFraction {
        YearFraction{
            val: (self.day_count(start, end) as f64) / ( self.days_per_year() as f64 )
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
