use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, uniffi::Object)]
pub struct WpUuid {
    inner: Uuid,
}

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
    use rstest::*;

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
        assert!(matches!(uuid.unwrap_err(), WpUuidParseError::NotVersion4));
    }

    #[rstest]
    fn test_invalid_uuid_error() {
        let uuid = WpUuid::parse("not uuid".to_string());
        assert!(matches!(uuid.unwrap_err(), WpUuidParseError::InvalidUuid));
    }
}
