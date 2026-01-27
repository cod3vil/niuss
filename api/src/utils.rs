use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,        // Subject (user ID)
    pub email: String,   // User email
    pub is_admin: bool,  // Admin role
    pub exp: i64,        // Expiration time
    pub iat: i64,        // Issued at
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Failed to hash password: {}", e))?
        .to_string();
    
    Ok(password_hash)
}

/// Verify a password against a hash using Argon2id
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| anyhow!("Failed to parse password hash: {}", e))?;
    
    let argon2 = Argon2::default();
    
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ============================================================================
// JWT Token Functions
// ============================================================================

/// Generate a JWT token for a user
pub fn generate_token(
    user_id: i64,
    email: &str,
    is_admin: bool,
    secret: &str,
    expiration_seconds: i64,
) -> Result<String> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expiration_seconds);
    
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        is_admin,
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("Failed to generate token: {}", e))?;
    
    Ok(token)
}

/// Verify and decode a JWT token
pub fn verify_token(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow!("Failed to verify token: {}", e))?;
    
    Ok(token_data.claims)
}

/// Generate a refresh token (longer expiration)
pub fn generate_refresh_token(
    user_id: i64,
    email: &str,
    is_admin: bool,
    secret: &str,
) -> Result<String> {
    // Refresh tokens expire in 7 days
    generate_token(user_id, email, is_admin, secret, 7 * 24 * 3600)
}

// ============================================================================
// Input Validation Functions
// ============================================================================

/// Validate email format (RFC 5322 compliant)
pub fn validate_email(email: &str) -> Result<()> {
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    ).unwrap();
    
    if !email_regex.is_match(email) {
        return Err(anyhow!("Invalid email format"));
    }
    
    if email.len() > 255 {
        return Err(anyhow!("Email is too long (max 255 characters)"));
    }
    
    Ok(())
}

/// Validate password strength
/// Requirements: At least 8 characters, contains letters and numbers
pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(anyhow!("Password must be at least 8 characters long"));
    }
    
    if password.len() > 128 {
        return Err(anyhow!("Password is too long (max 128 characters)"));
    }
    
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());
    
    if !has_letter || !has_number {
        return Err(anyhow!("Password must contain both letters and numbers"));
    }
    
    Ok(())
}

/// Generate a random referral code
pub fn generate_referral_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate a unique subscription token (64 characters)
pub fn generate_subscription_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate a secure node secret (32 characters)
pub fn generate_node_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ============================================================================
// XSS Prevention Functions
// ============================================================================

/// Sanitize user input to prevent XSS attacks
/// Escapes HTML special characters
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

/// Validate and sanitize string input
/// Removes control characters and limits length
pub fn sanitize_string(input: &str, max_length: usize) -> Result<String> {
    // Remove control characters (except newline and tab)
    let sanitized: String = input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    
    // Check length
    if sanitized.len() > max_length {
        return Err(anyhow!("Input exceeds maximum length of {} characters", max_length));
    }
    
    Ok(sanitized)
}

/// Validate numeric input is within range
pub fn validate_numeric_range(value: i64, min: i64, max: i64, field_name: &str) -> Result<()> {
    if value < min || value > max {
        return Err(anyhow!("{} must be between {} and {}", field_name, min, max));
    }
    Ok(())
}

/// Validate port number
pub fn validate_port(port: i32) -> Result<()> {
    if port < 1 || port > 65535 {
        return Err(anyhow!("Port must be between 1 and 65535"));
    }
    Ok(())
}

/// Validate hostname/IP address format
pub fn validate_host(host: &str) -> Result<()> {
    if host.is_empty() {
        return Err(anyhow!("Host cannot be empty"));
    }
    
    if host.len() > 253 {
        return Err(anyhow!("Host is too long (max 253 characters)"));
    }
    
    // Basic validation - should be alphanumeric with dots, hyphens, or valid IP
    let valid_chars = host.chars().all(|c| {
        c.is_alphanumeric() || c == '.' || c == '-' || c == ':'
    });
    
    if !valid_chars {
        return Err(anyhow!("Host contains invalid characters"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_creates_valid_hash() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        
        // Argon2 hash should start with $argon2
        assert!(hash.starts_with("$argon2"));
        
        // Hash should be reasonably long
        assert!(hash.len() > 50);
    }

    #[test]
    fn test_verify_password_with_correct_password() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();
        
        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_with_incorrect_password() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();
        
        let result = verify_password("wrong_password", &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_hash_password_generates_different_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        
        // Same password should generate different hashes due to different salts
        assert_ne!(hash1, hash2);
        
        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_verify_password_with_empty_password() {
        let password = "";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password("", &hash).unwrap());
        assert!(!verify_password("not_empty", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_with_special_characters() {
        let password = "p@ssw0rd!#$%^&*()";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("p@ssw0rd", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_with_unicode() {
        let password = "密码测试123";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("密码测试", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_with_invalid_hash() {
        let result = verify_password("password", "invalid_hash");
        assert!(result.is_err());
    }

    // JWT token tests
    
    #[test]
    fn test_generate_token_creates_valid_token() {
        let secret = "test_secret_key";
        let token = generate_token(1, "test@example.com", false, secret, 3600).unwrap();
        
        // Token should be a non-empty string
        assert!(!token.is_empty());
        
        // Token should have 3 parts separated by dots (header.payload.signature)
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn test_verify_token_with_valid_token() {
        let secret = "test_secret_key";
        let user_id = 123;
        let email = "user@example.com";
        let is_admin = true;
        
        let token = generate_token(user_id, email, is_admin, secret, 3600).unwrap();
        let claims = verify_token(&token, secret).unwrap();
        
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.is_admin, is_admin);
    }

    #[test]
    fn test_verify_token_with_invalid_secret() {
        let secret = "test_secret_key";
        let wrong_secret = "wrong_secret_key";
        
        let token = generate_token(1, "test@example.com", false, secret, 3600).unwrap();
        let result = verify_token(&token, wrong_secret);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_with_malformed_token() {
        let secret = "test_secret_key";
        let result = verify_token("invalid.token.format", secret);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_refresh_token() {
        let secret = "test_secret_key";
        let token = generate_refresh_token(1, "test@example.com", false, secret).unwrap();
        
        // Verify the token is valid
        let claims = verify_token(&token, secret).unwrap();
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.email, "test@example.com");
        
        // Verify expiration is approximately 7 days from now
        let now = Utc::now().timestamp();
        let expected_exp = now + (7 * 24 * 3600);
        assert!((claims.exp - expected_exp).abs() < 5); // Within 5 seconds tolerance
    }

    #[test]
    fn test_token_contains_correct_claims() {
        let secret = "test_secret_key";
        let user_id = 456;
        let email = "admin@example.com";
        let is_admin = true;
        
        let token = generate_token(user_id, email, is_admin, secret, 3600).unwrap();
        let claims = verify_token(&token, secret).unwrap();
        
        // Verify all claims
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.is_admin, is_admin);
        
        // Verify iat (issued at) is recent
        let now = Utc::now().timestamp();
        assert!((claims.iat - now).abs() < 5);
        
        // Verify exp (expiration) is in the future
        assert!(claims.exp > now);
    }

    #[test]
    fn test_different_users_get_different_tokens() {
        let secret = "test_secret_key";
        
        let token1 = generate_token(1, "user1@example.com", false, secret, 3600).unwrap();
        let token2 = generate_token(2, "user2@example.com", false, secret, 3600).unwrap();
        
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_admin_and_regular_user_tokens() {
        let secret = "test_secret_key";
        
        let admin_token = generate_token(1, "admin@example.com", true, secret, 3600).unwrap();
        let user_token = generate_token(2, "user@example.com", false, secret, 3600).unwrap();
        
        let admin_claims = verify_token(&admin_token, secret).unwrap();
        let user_claims = verify_token(&user_token, secret).unwrap();
        
        assert!(admin_claims.is_admin);
        assert!(!user_claims.is_admin);
    }

    // Input validation tests
    
    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user@example.com").is_ok());
        assert!(validate_email("user+tag@example.co.uk").is_ok());
        assert!(validate_email("user_123@test-domain.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user @example.com").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn test_validate_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        assert!(validate_email(&long_email).is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("Test1234").is_ok());
        assert!(validate_password("abcdefgh1").is_ok());
        assert!(validate_password("P@ssw0rd!").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        assert!(validate_password("pass1").is_err());
        assert!(validate_password("abc123").is_err());
    }

    #[test]
    fn test_validate_password_no_letters() {
        assert!(validate_password("12345678").is_err());
    }

    #[test]
    fn test_validate_password_no_numbers() {
        assert!(validate_password("password").is_err());
        assert!(validate_password("abcdefgh").is_err());
    }

    #[test]
    fn test_validate_password_too_long() {
        let long_password = format!("{}1", "a".repeat(130));
        assert!(validate_password(&long_password).is_err());
    }

    #[test]
    fn test_generate_referral_code() {
        let code1 = generate_referral_code();
        let code2 = generate_referral_code();
        
        // Should be 8 characters
        assert_eq!(code1.len(), 8);
        assert_eq!(code2.len(), 8);
        
        // Should be different (very high probability)
        assert_ne!(code1, code2);
        
        // Should only contain uppercase letters and numbers
        assert!(code1.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        assert!(code2.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    // XSS Prevention tests
    
    #[test]
    fn test_sanitize_html_escapes_special_characters() {
        assert_eq!(sanitize_html("<script>alert('xss')</script>"), 
                   "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;");
        assert_eq!(sanitize_html("Hello & goodbye"), "Hello &amp; goodbye");
        assert_eq!(sanitize_html("<div>Test</div>"), "&lt;div&gt;Test&lt;&#x2F;div&gt;");
        assert_eq!(sanitize_html("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_sanitize_html_with_safe_text() {
        assert_eq!(sanitize_html("Hello World"), "Hello World");
        assert_eq!(sanitize_html("123456"), "123456");
        assert_eq!(sanitize_html("test@example.com"), "test@example.com");
    }

    #[test]
    fn test_sanitize_string_removes_control_characters() {
        let input = "Hello\x00World\x01Test";
        let result = sanitize_string(input, 100).unwrap();
        assert_eq!(result, "HelloWorldTest");
    }

    #[test]
    fn test_sanitize_string_preserves_newlines_and_tabs() {
        let input = "Hello\nWorld\tTest";
        let result = sanitize_string(input, 100).unwrap();
        assert_eq!(result, "Hello\nWorld\tTest");
    }

    #[test]
    fn test_sanitize_string_enforces_max_length() {
        let input = "a".repeat(200);
        let result = sanitize_string(&input, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_numeric_range_valid() {
        assert!(validate_numeric_range(50, 0, 100, "value").is_ok());
        assert!(validate_numeric_range(0, 0, 100, "value").is_ok());
        assert!(validate_numeric_range(100, 0, 100, "value").is_ok());
    }

    #[test]
    fn test_validate_numeric_range_invalid() {
        assert!(validate_numeric_range(-1, 0, 100, "value").is_err());
        assert!(validate_numeric_range(101, 0, 100, "value").is_err());
    }

    #[test]
    fn test_validate_port_valid() {
        assert!(validate_port(80).is_ok());
        assert!(validate_port(443).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(1).is_ok());
        assert!(validate_port(65535).is_ok());
    }

    #[test]
    fn test_validate_port_invalid() {
        assert!(validate_port(0).is_err());
        assert!(validate_port(-1).is_err());
        assert!(validate_port(65536).is_err());
        assert!(validate_port(100000).is_err());
    }

    #[test]
    fn test_validate_host_valid() {
        assert!(validate_host("example.com").is_ok());
        assert!(validate_host("sub.example.com").is_ok());
        assert!(validate_host("192.168.1.1").is_ok());
        assert!(validate_host("localhost").is_ok());
        assert!(validate_host("my-server.com").is_ok());
    }

    #[test]
    fn test_validate_host_invalid() {
        assert!(validate_host("").is_err());
        assert!(validate_host("invalid host").is_err());
        assert!(validate_host("host@example").is_err());
        
        // Too long
        let long_host = "a".repeat(300);
        assert!(validate_host(&long_host).is_err());
    }

    // Property-based tests
    use proptest::prelude::*;

    // Feature: vpn-subscription-platform, Property 18: 密码哈希不可逆性
    // **Validates: Requirements 14.1**
    // For any user password, the stored value should be a hash of the password rather than plaintext,
    // and the hash should not be reversible to derive the original password
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_password_hash_irreversibility(
            password in "[A-Za-z0-9!@#$%^&*()_+\\-=\\[\\]{};':\"\\\\|,.<>/?]{8,32}"
        ) {
            // Hash the password
            let hash = hash_password(&password).unwrap();
            
            // Property 1: Hash should not equal the original password
            prop_assert_ne!(&hash, &password);
            
            // Property 2: Hash should start with $argon2 (Argon2 format)
            prop_assert!(hash.starts_with("$argon2"));
            
            // Property 3: Hash should be significantly longer than typical passwords
            prop_assert!(hash.len() > 50);
            
            // Property 4: Hash should verify correctly with the original password
            prop_assert!(verify_password(&password, &hash).unwrap());
            
            // Property 5: Hash should not verify with a different password
            let different_password = format!("{}x", password);
            prop_assert!(!verify_password(&different_password, &hash).unwrap());
            
            // Property 6: Same password should produce different hashes (due to salt)
            let hash2 = hash_password(&password).unwrap();
            prop_assert_ne!(&hash, &hash2);
            
            // Property 7: Both hashes should verify with the original password
            prop_assert!(verify_password(&password, &hash2).unwrap());
        }
    }

    // Additional property test: Password hash should not contain the original password
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_password_not_in_hash(
            password in "[A-Za-z0-9]{8,16}"
        ) {
            let hash = hash_password(&password).unwrap();
            
            // The hash should not contain the original password as a substring
            prop_assert!(!hash.contains(&password));
        }
    }

    // Property test: Empty and very long passwords should be handled correctly
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_password_edge_cases(
            password in prop::string::string_regex(".*").unwrap()
        ) {
            // Should be able to hash any string
            let hash_result = hash_password(&password);
            prop_assert!(hash_result.is_ok());
            
            if let Ok(hash) = hash_result {
                // Should verify correctly
                prop_assert!(verify_password(&password, &hash).unwrap());
            }
        }
    }

    // Property test: Unicode passwords should work correctly
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_unicode_password_hashing(
            password in "[\\u{4e00}-\\u{9fff}]{4,16}" // Chinese characters
        ) {
            let hash = hash_password(&password).unwrap();
            
            // Should verify correctly
            prop_assert!(verify_password(&password, &hash).unwrap());
            
            // Should not verify with different unicode string
            let different = format!("{}a", password);
            prop_assert!(!verify_password(&different, &hash).unwrap());
        }
    }

    // Property test: Hash consistency - same password always verifies
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_hash_verification_consistency(
            password in "[A-Za-z0-9!@#$%^&*]{8,20}",
            num_verifications in 1..=10usize
        ) {
            let hash = hash_password(&password).unwrap();
            
            // Verify multiple times - should always succeed
            for _ in 0..num_verifications {
                prop_assert!(verify_password(&password, &hash).unwrap());
            }
        }
    }

    // Feature: vpn-subscription-platform, Property 2: 认证令牌有效性
    // **Validates: Requirements 1.3, 1.4, 14.2, 14.3**
    // For any user, logging in with correct credentials should return a valid JWT token,
    // and using that token to access protected resources should succeed;
    // logging in with incorrect credentials should be rejected
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_authentication_token_validity(
            user_id in 1..=1000000i64,
            email in "[a-z]{5,10}@[a-z]{3,8}\\.com",
            is_admin in prop::bool::ANY,
            expiration in 60..=86400i64,
        ) {
            let secret = "test_secret_key_for_property_test";
            
            // Property 1: Token generation should succeed for valid inputs
            let token_result = generate_token(user_id, &email, is_admin, secret, expiration);
            prop_assert!(token_result.is_ok());
            
            let token = token_result.unwrap();
            
            // Property 2: Token should be non-empty
            prop_assert!(!token.is_empty());
            
            // Property 3: Token should have JWT format (3 parts separated by dots)
            prop_assert_eq!(token.split('.').count(), 3);
            
            // Property 4: Token verification should succeed with correct secret
            let claims_result = verify_token(&token, secret);
            prop_assert!(claims_result.is_ok());
            
            let claims = claims_result.unwrap();
            
            // Property 5: Claims should match the original user data
            prop_assert_eq!(claims.sub, user_id);
            prop_assert_eq!(claims.email, email);
            prop_assert_eq!(claims.is_admin, is_admin);
            
            // Property 6: Token verification should fail with wrong secret
            let wrong_secret = "wrong_secret_key";
            let wrong_claims_result = verify_token(&token, wrong_secret);
            prop_assert!(wrong_claims_result.is_err());
            
            // Property 7: Expiration should be in the future
            let now = Utc::now().timestamp();
            prop_assert!(claims.exp > now);
            
            // Property 8: Issued at should be recent (within last 5 seconds)
            prop_assert!((claims.iat - now).abs() < 5);
        }
    }

    // Property test: Token round-trip consistency
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_token_roundtrip_consistency(
            user_id in 1..=1000000i64,
            email in "[a-z]{5,10}@[a-z]{3,8}\\.com",
            is_admin in prop::bool::ANY,
        ) {
            let secret = "test_secret_for_roundtrip";
            
            // Generate token
            let token = generate_token(user_id, &email, is_admin, secret, 3600).unwrap();
            
            // Verify and extract claims
            let claims = verify_token(&token, secret).unwrap();
            
            // All user data should survive the round-trip
            prop_assert_eq!(claims.sub, user_id);
            prop_assert_eq!(claims.email, email);
            prop_assert_eq!(claims.is_admin, is_admin);
        }
    }

    // Property test: Admin vs regular user tokens
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_admin_role_preservation(
            user_id in 1..=1000000i64,
            email in "[a-z]{5,10}@[a-z]{3,8}\\.com",
            is_admin in prop::bool::ANY,
        ) {
            let secret = "test_secret";
            
            let token = generate_token(user_id, &email, is_admin, secret, 3600).unwrap();
            let claims = verify_token(&token, secret).unwrap();
            
            // Admin role should be preserved exactly
            prop_assert_eq!(claims.is_admin, is_admin);
        }
    }

    // Property test: Refresh tokens have longer expiration
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        
        #[test]
        fn test_refresh_token_expiration(
            user_id in 1..=1000000i64,
            email in "[a-z]{5,10}@[a-z]{3,8}\\.com",
            is_admin in prop::bool::ANY,
        ) {
            let secret = "test_secret";
            
            // Generate regular token (1 hour)
            let regular_token = generate_token(user_id, &email, is_admin, secret, 3600).unwrap();
            let regular_claims = verify_token(&regular_token, secret).unwrap();
            
            // Generate refresh token (7 days)
            let refresh_token = generate_refresh_token(user_id, &email, is_admin, secret).unwrap();
            let refresh_claims = verify_token(&refresh_token, secret).unwrap();
            
            // Refresh token should expire much later than regular token
            prop_assert!(refresh_claims.exp > regular_claims.exp);
            
            // Refresh token should expire approximately 7 days from now
            let now = Utc::now().timestamp();
            let expected_refresh_exp = now + (7 * 24 * 3600);
            prop_assert!((refresh_claims.exp - expected_refresh_exp).abs() < 10);
        }
    }
}
