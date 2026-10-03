use regex::Regex;

pub fn directory_mode_validator() -> Regex {
    regex::Regex::new("^0*[0-7]{3}$").unwrap()
}

pub fn directory_mode_typing_validator() -> Regex {
    regex::Regex::new("^[0-7]+$").unwrap()
}

#[cfg(test)]
mod tests {
    use test_base::init_logs;

    use super::*;

    #[test]
    fn test_directory_mode_validator() {
        init_logs();

        let validator = directory_mode_validator();
        assert!(validator.is_match("0755"));
        assert!(validator.is_match("755"));
        assert!(!validator.is_match("75"));
        assert!(!validator.is_match("758"));
        assert!(!validator.is_match("75733"));
        assert!(!validator.is_match("7777"));
    }

    #[test]
    fn test_directory_mode_typing_validator() {
        init_logs();

        let validator = directory_mode_typing_validator();
        assert!(validator.is_match("0755"));
        assert!(validator.is_match("755"));
        assert!(!validator.is_match("8"));
        assert!(!validator.is_match("9"));
        assert!(!validator.is_match("79"));
    }
}
