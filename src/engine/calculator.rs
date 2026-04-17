use rust_decimal::Decimal;

pub fn calculate_emission(quantity: Decimal, factor_value: Decimal) -> Decimal {
    return quantity * factor_value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_calculate_emission_transport() {
        let quantity = dec!(15.5);
        let factor = dec!(0.112);
        let result = calculate_emission(quantity, factor);

        assert_eq!(result, dec!(1.736));
    }

    #[test]
    fn test_calculate_emission_food() {
        let quantity = dec!(0.2);
        let factor = dec!(27.0);
        let result = calculate_emission(quantity, factor);

        assert_eq!(result, dec!(5.4));
    }
}
