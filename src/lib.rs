use bdays::date::Date;

pub mod daycount;
pub mod rate;

pub trait IRCurve {

    //fn asof_date(&self) -> Date;

    fn zero_rate(&self, maturity: Date) -> f64;

}
