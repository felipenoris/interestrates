use crate::daycount::YearFraction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compounding {
    Continuous,
    Simple,
    Exponential,
}

#[derive(Debug, Clone, Copy)]
pub struct Rate {
    compounding: Compounding,
    val: f64, // annual interest rate
}

/// Annual interest rate.
impl Rate {

    pub fn new(
        compounding: Compounding,
        val: f64,
    ) -> Self {
        Rate {
            compounding,
            val,
        }
    }

    pub fn from_pct(
        compounding: Compounding,
        pct: f64,
    ) -> Self {
        Self::new(compounding, pct / 100.0)
    }

    pub fn from_factor(compounding: Compounding, factor: f64, yf: YearFraction) -> Self {
        let t = yf.value();

        let r = match compounding {
                Compounding::Continuous => factor.ln() / t,
                Compounding::Simple => (factor - 1.0) / t,
                Compounding::Exponential => factor.powf(1.0 / t) - 1.0,
        };

        Rate{
            val: r,
            compounding,
        }
    }

    pub fn from_discount(compounding: Compounding, discount: f64, yf: YearFraction) -> Self {
        Self::from_factor(compounding, 1.0 / discount, yf)
    }

    /// 0.10 means 10%
    pub fn value(&self) -> f64 {
        self.val
    }

    pub fn compounding(&self) -> Compounding {
        self.compounding
    }

    /// 10.25 means 12.25%
    pub fn value_as_pct(&self) -> f64 {
        self.val * 100.0
    }

    pub fn factor(&self, yf: YearFraction) -> f64 {
        if yf.value() == 0.0 {
            1.0
        } else {

            let r = self.value();
            let t = yf.value();

            match self.compounding {
                Compounding::Continuous => (r * t).exp(),
                Compounding::Simple => 1.0 + r * t,
                Compounding::Exponential => (1.0 + r).powf(t),
            }
        }
    }

    pub fn discount(&self, yf: YearFraction) -> f64 {
        1.0 / self.factor(yf)
    }
}

pub struct RateYF {
    rate: Rate,
    year_fraction: YearFraction,
}

impl RateYF {

    pub fn new(rate: Rate, year_fraction: YearFraction) -> Self {
        Self { rate, year_fraction }
    }

    pub fn rate(&self) -> Rate {
        self.rate
    }

    pub fn value(&self) -> f64 {
        self.rate.value()
    }

    pub fn value_as_pct(&self) -> f64 {
        self.rate.value_as_pct()
    }

    pub fn factor(&self) -> f64 {
        self.rate.factor(self.year_fraction)
    }

    pub fn discount(&self) -> f64 {
        1.0 / self.factor()
    }

    pub fn year_fraction(&self) -> YearFraction {
        self.year_fraction
    }

    pub fn effective_rate_value(&self) -> f64 {
        self.factor() - 1.0
    }

    pub fn effective_rate_value_as_pct(&self) -> f64 {
        self.effective_rate_value() * 100.0
    }
}
