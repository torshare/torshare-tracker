/// Extracts the first value from a separator-separated list of values.
pub fn get_first_value(input: &str, separator: char) -> &str {
    match input.splitn(2, separator).next() {
        Some(value) => value,
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_first_value() {
        assert_eq!(get_first_value("a,b,c", ','), "a");
        assert_eq!(get_first_value("a", ','), "a");
        assert_eq!(get_first_value("", ','), "");
    }
}
