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
    annual_rate: f64, // annual interest rate, 0.123 means 12.30%
    year_fraction: YearFraction,
}

impl Rate {

    pub fn from_annual_rate(
        compounding: Compounding,
        annual_rate: f64,
        year_fraction: YearFraction,
    ) -> Self {
        Self {
            compounding,
            annual_rate,
            year_fraction,
        }
    }

    pub fn from_annual_rate_pct(
        compounding: Compounding,
        annual_rate_pct: f64,
        year_fraction: YearFraction,
    ) -> Self {
        Self::from_annual_rate(
            compounding,
            annual_rate_pct / 100.0,
            year_fraction,
        )
    }

    pub fn from_factor(
        compounding: Compounding,
        factor: f64,
        year_fraction: YearFraction,
    ) -> Self {

        let t = year_fraction.value();

        let r = match compounding {
                Compounding::Continuous => factor.ln() / t,
                Compounding::Simple => (factor - 1.0) / t,
                Compounding::Exponential => factor.powf(1.0 / t) - 1.0,
        };

        Self::from_annual_rate(
            compounding,
            r,
            year_fraction,
        )
    }

    pub fn from_discount(
        compounding: Compounding,
        discount: f64,
        year_fraction: YearFraction,
    ) -> Self {
        Self::from_factor(
            compounding,
            1.0 / discount,
            year_fraction,
        )
    }

    pub fn annual_rate(&self) -> f64 {
        self.annual_rate
    }

    pub fn annual_rate_pct(&self) -> f64 {
        self.annual_rate() * 100.0
    }

    pub fn factor(&self) -> f64 {
        if self.year_fraction.value() == 0.0 || self.annual_rate == 0.0 {
            1.0
        } else {

            let r = self.annual_rate;
            let t = self.year_fraction.value();

            match self.compounding {
                Compounding::Continuous => (r * t).exp(),
                Compounding::Simple => 1.0 + r * t,
                Compounding::Exponential => (1.0 + r).powf(t),
            }
        }
    }

    pub fn discount(&self) -> f64 {
        1.0 / self.factor()
    }

    pub fn year_fraction(&self) -> YearFraction {
        self.year_fraction
    }

    pub fn compounding(&self) -> Compounding {
        self.compounding
    }

    pub fn effective_rate(&self) -> f64 {
        self.factor() - 1.0
    }

    pub fn effective_rate_pct(&self) -> f64 {
        self.effective_rate() * 100.0
    }
}
