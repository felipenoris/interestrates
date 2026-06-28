use crate::daycount::YearFraction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compounding {
    Continuous,
    Simple,
    Exponential,
}

#[derive(Debug, Clone, Copy)]
pub struct Rate {
    val: f64,
    compounding: Compounding,
}

impl Rate {

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