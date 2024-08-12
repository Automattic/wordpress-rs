use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, uniffi::Object)]
pub struct WpUuid {
    inner: Uuid,
}

#[uniffi::export]
impl WpUuid {
    #[uniffi::constructor]
    pub fn new() -> Self {
        // See https://github.com/WordPress/wordpress-develop/blob/6.6.1/src/wp-includes/functions.php#L7899-L7916
        Self {
            inner: Uuid::new_v4(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error, uniffi::Error)]
pub enum WpUuidParseError {
    #[error("Invalid UUID string")]
    InvalidUuid,

    #[error("Not a version 4 UUID")]
    NotVersion4,
}

#[uniffi::export]
impl WpUuid {
    #[uniffi::constructor]
    pub fn parse(input: String) -> Result<Self, WpUuidParseError> {
        let uuid = Uuid::parse_str(input.as_str()).map_err(|_| WpUuidParseError::InvalidUuid)?;
        if uuid.get_version_num() != 4 {
            return Err(WpUuidParseError::NotVersion4);
        }
        Ok(Self { inner: uuid })
    }

    pub fn uuid_string(&self) -> String {
        self.inner.hyphenated().to_string()
    }
}

impl Default for WpUuid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use rstest::*;

    // Copied from WordPress Core:
    // https://github.com/WordPress/wordpress-develop/blob/471a619/src/wp-includes/functions.php#L7912
    static WP_DEFAULT_REGEX: &str =
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";
    static WP_V4_REGEX: &str =
        r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";

    #[rstest]
    #[case("caa8b54a-eb5e-4134-8ae2-a3946a428ec7")]
    #[case("550e8400-e29b-41d4-a716-446655440000")]
    #[case("16fd2706-8baf-433b-82eb-8c7fada847da")]
    #[case("f47ac10b-58cc-4372-a567-0e02b2c3d479")]
    #[case("20354d7a-e4fe-47af-8ff6-187bca92f3f9")]
    fn test_parse_v4_uuid(#[case] uuid_str: String) {
        let uuid = WpUuid::parse(uuid_str.to_string()).unwrap();
        assert_eq!(uuid.uuid_string(), uuid_str);
    }

    #[rstest]
    #[case("6ba7b810-9dad-11d1-80b4-00c04fd430c8")]
    #[case("01020304-0506-0210-0800-0026b9777788")]
    #[case("f47ac10b-58cc-3372-a567-0e02b2c3d479")]
    #[case("257d1bda-5c8a-5e34-85c4-b06eb6a6ab5a")]
    #[case("c7b130a6-9983-2e8f-b5d2-006e09d3b8a3")]
    fn test_parse_non_v4_uuid(#[case] uuid_str: String) {
        let uuid = WpUuid::parse(uuid_str);
        assert_eq!(uuid.unwrap_err(), WpUuidParseError::NotVersion4);
    }

    #[rstest]
    #[case("CAA8B54A-EB5E-4134-8AE2-A3946A428EC7")]
    #[case("550E8400-e29b-41d4-A716-446655440000")]
    #[case("16fd27068baf433b82eb8c7fada847da")]
    #[case("F47AC10B58CC4372A5670E02B2C3D479")]
    fn test_parse_other_formats(#[case] uuid_str: String) {
        let uuid = WpUuid::parse(uuid_str.to_string()).unwrap();
        let uuid = uuid.uuid_string();
        assert!(Regex::new(WP_DEFAULT_REGEX)
            .unwrap()
            .is_match(uuid.as_str()));
        assert!(Regex::new(WP_V4_REGEX).unwrap().is_match(uuid.as_str()));
    }

    #[rstest]
    fn test_new_uuid_is_compatible_with_wordpress() {
        let uuid = WpUuid::new().uuid_string();
        assert!(Regex::new(WP_DEFAULT_REGEX)
            .unwrap()
            .is_match(uuid.as_str()));
        assert!(Regex::new(WP_V4_REGEX).unwrap().is_match(uuid.as_str()));
    }

    #[rstest]
    fn test_invalid_uuid_error() {
        let uuid = WpUuid::parse("not uuid".to_string());
        assert_eq!(uuid.unwrap_err(), WpUuidParseError::InvalidUuid);
    }
}
