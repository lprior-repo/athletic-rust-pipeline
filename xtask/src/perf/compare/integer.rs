use anyhow::{Context, Result};

pub(super) struct Tolerance {
    fraction: f64,
    numerator: u128,
    denominator: u128,
}

impl Tolerance {
    pub(super) fn parse(fraction: f64) -> Result<Self> {
        super::validate_tolerance(fraction)?;
        let (numerator, denominator) = if fraction < 1e-20 {
            (0, 1)
        } else {
            let decimal = fraction.to_string();
            let digits = decimal
                .strip_prefix("0.")
                .context("validated tolerance has no fractional decimal representation")?;
            let numerator = digits.parse::<u128>()?;
            let scale = u32::try_from(digits.len())?;
            let denominator = 10_u128
                .checked_pow(scale)
                .context("memory tolerance decimal scale overflowed")?;
            (numerator, denominator)
        };
        Ok(Self {
            fraction,
            numerator,
            denominator,
        })
    }

    pub(super) fn fraction(&self) -> f64 {
        self.fraction
    }

    pub(super) fn integer_exceeded(&self, baseline: u64, current: u64) -> Result<bool> {
        let extra = u128::from(baseline)
            .checked_mul(self.numerator)
            .context("memory tolerance numerator overflowed")?
            .checked_div(self.denominator)
            .context("memory tolerance denominator is zero")?;
        let limit = u128::from(baseline)
            .checked_add(extra)
            .context("memory tolerance limit overflowed")?;
        Ok(u128::from(current) > limit)
    }
}
