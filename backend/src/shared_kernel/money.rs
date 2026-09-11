use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Usd,
    Eur,
    Gbp,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Currency::Usd => "USD",
            Currency::Eur => "EUR",
            Currency::Gbp => "GBP",
        };
        write!(f, "{code}")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("money amount cannot be negative, got {0}")]
    Negative(Decimal),
    #[error("cannot combine {0} with {1}: currencies do not match")]
    CurrencyMismatch(Currency, Currency),
    #[error("arithmetic overflow while operating on money values")]
    Overflow,
}

/// Immutable value object representing a monetary amount in a specific
/// currency. Every operation is checked — there is no path that panics or
/// silently wraps/truncates on overflow or a negative result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: Currency) -> Result<Self, MoneyError> {
        if amount.is_sign_negative() && !amount.is_zero() {
            return Err(MoneyError::Negative(amount));
        }
        Ok(Self { amount, currency })
    }

    pub fn zero(currency: Currency) -> Self {
        Self { amount: Decimal::ZERO, currency }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn checked_add(&self, other: &Money) -> Result<Money, MoneyError> {
        self.assert_same_currency(other)?;
        let amount = self.amount.checked_add(other.amount).ok_or(MoneyError::Overflow)?;
        Money::new(amount, self.currency)
    }

    pub fn checked_sub(&self, other: &Money) -> Result<Money, MoneyError> {
        self.assert_same_currency(other)?;
        let amount = self.amount.checked_sub(other.amount).ok_or(MoneyError::Overflow)?;
        Money::new(amount, self.currency)
    }

    pub fn checked_mul_qty(&self, quantity: i64) -> Result<Money, MoneyError> {
        let amount = self
            .amount
            .checked_mul(Decimal::from(quantity))
            .ok_or(MoneyError::Overflow)?;
        Money::new(amount, self.currency)
    }

    fn assert_same_currency(&self, other: &Money) -> Result<(), MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(self.currency, other.currency));
        }
        Ok(())
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_negative_amounts() {
        assert_eq!(
            Money::new(Decimal::from(-1), Currency::Usd),
            Err(MoneyError::Negative(Decimal::from(-1)))
        );
    }

    #[test]
    fn rejects_cross_currency_addition() {
        let usd = Money::new(Decimal::from(10), Currency::Usd).unwrap();
        let eur = Money::new(Decimal::from(10), Currency::Eur).unwrap();
        assert_eq!(
            usd.checked_add(&eur),
            Err(MoneyError::CurrencyMismatch(Currency::Usd, Currency::Eur))
        );
    }
}
