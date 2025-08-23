#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::*;
    use crate::{ApiError, MomentKind};
    use rust_decimal::Decimal;
    use serde_json;

    mod validation_tests {
        use super::*;

        #[test]
        fn test_validate_wallet_address_valid() {
            // Valid Solana addresses
            let valid_addresses = [
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
                "So11111111111111111111111111111111111111112",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
            ];

            for addr in &valid_addresses {
                assert!(
                    validate_wallet_address(addr).is_ok(),
                    "Failed for address: {}",
                    addr
                );
            }
        }

        #[test]
        fn test_validate_wallet_address_invalid() {
            // Invalid addresses
            let invalid_addresses = [
                "",                                              // Empty
                "abc",                                           // Too short
                "0x1234567890123456789012345678901234567890",    // Ethereum address
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM0", // Too long
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWW0",  // Contains invalid character '0'
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWI",  // Contains invalid character 'I'
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWO",  // Contains invalid character 'O'
                "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWl",  // Contains invalid character 'l'
            ];

            for addr in &invalid_addresses {
                assert!(
                    validate_wallet_address(addr).is_err(),
                    "Should fail for address: {}",
                    addr
                );
                match validate_wallet_address(addr) {
                    Err(ApiError::InvalidWallet) => {} // Expected
                    _ => panic!("Expected InvalidWallet error for: {}", addr),
                }
            }
        }

        #[test]
        fn test_validate_pagination_defaults() {
            let (limit, cursor) = validate_pagination(None, None).unwrap();
            assert_eq!(limit, crate::constants::api::DEFAULT_PAGE_SIZE);
            assert_eq!(cursor, None);
        }

        #[test]
        fn test_validate_pagination_custom_limit() {
            let (limit, cursor) = validate_pagination(Some(50), Some("test_cursor")).unwrap();
            assert_eq!(limit, 50);
            assert_eq!(cursor, Some("test_cursor".to_string()));
        }

        #[test]
        fn test_validate_pagination_max_limit() {
            let (limit, _) = validate_pagination(Some(9999), None).unwrap();
            assert_eq!(limit, crate::constants::api::MAX_PAGE_SIZE);
        }

        #[test]
        fn test_validate_pagination_zero_limit() {
            let result = validate_pagination(Some(0), None);
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::BadRequest(msg) => assert!(msg.contains("greater than 0")),
                _ => panic!("Expected BadRequest error"),
            }
        }

        #[test]
        fn test_validate_moment_kinds_valid() {
            let kinds = validate_moment_kinds(Some("S2E,BHD,BadRoute")).unwrap();
            assert_eq!(kinds.len(), 3);
            assert!(kinds.contains(&MomentKind::SoldTooEarly));
            assert!(kinds.contains(&MomentKind::BagHolderDrawdown));
            assert!(kinds.contains(&MomentKind::BadRoute));
        }

        #[test]
        fn test_validate_moment_kinds_empty() {
            let kinds = validate_moment_kinds(None).unwrap();
            assert!(kinds.is_empty());
        }

        #[test]
        fn test_validate_moment_kinds_invalid() {
            let result = validate_moment_kinds(Some("S2E,InvalidKind,BHD"));
            assert!(result.is_err());
            match result.unwrap_err() {
                ApiError::BadRequest(msg) => assert!(msg.contains("Invalid moment kind")),
                _ => panic!("Expected BadRequest error"),
            }
        }

        #[test]
        fn test_validate_moment_kinds_with_whitespace() {
            let kinds = validate_moment_kinds(Some(" S2E , BHD , BadRoute ")).unwrap();
            assert_eq!(kinds.len(), 3);
            assert!(kinds.contains(&MomentKind::SoldTooEarly));
        }
    }

    mod decimal_serde_tests {
        use super::*;

        #[derive(serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            #[serde(with = "DecimalSerde")]
            value: Decimal,
        }

        #[test]
        fn test_decimal_serialize() {
            let test = TestStruct {
                value: Decimal::new(12345, 2), // 123.45
            };
            let json = serde_json::to_string(&test).unwrap();
            assert_eq!(json, r#"{"value":"123.45"}"#);
        }

        #[test]
        fn test_decimal_deserialize() {
            let json = r#"{"value":"123.45"}"#;
            let test: TestStruct = serde_json::from_str(json).unwrap();
            assert_eq!(test.value, Decimal::new(12345, 2));
        }

        #[test]
        fn test_decimal_roundtrip() {
            let original = TestStruct {
                value: Decimal::new(999999999, 6), // 999.999999
            };
            let json = serde_json::to_string(&original).unwrap();
            let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
            assert_eq!(original.value, deserialized.value);
        }

        #[test]
        fn test_decimal_deserialize_invalid() {
            let json = r#"{"value":"not_a_number"}"#;
            let result: Result<TestStruct, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }
    }

    mod result_ext_tests {
        use super::*;

        #[test]
        fn test_or_500_success() {
            let result: Result<i32, &str> = Ok(42);
            let api_result = result.or_500().unwrap();
            assert_eq!(api_result, 42);
        }

        #[test]
        fn test_or_500_error() {
            let result: Result<i32, &str> = Err("test error");
            let api_result = result.or_500();
            assert!(api_result.is_err());
            match api_result.unwrap_err() {
                ApiError::Internal(_) => {} // Expected
                _ => panic!("Expected Internal error"),
            }
        }

        #[test]
        fn test_or_bad_request_success() {
            let result: Result<i32, &str> = Ok(42);
            let api_result = result.or_bad_request("Custom message").unwrap();
            assert_eq!(api_result, 42);
        }

        #[test]
        fn test_or_bad_request_error() {
            let result: Result<i32, &str> = Err("test error");
            let api_result = result.or_bad_request("Custom message");
            assert!(api_result.is_err());
            match api_result.unwrap_err() {
                ApiError::BadRequest(msg) => assert_eq!(msg, "Custom message"),
                _ => panic!("Expected BadRequest error"),
            }
        }
    }
}
