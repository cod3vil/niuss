use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use redis::aio::ConnectionManager;
use serde_json::json;
use std::sync::Arc;

use crate::utils::{verify_token, Claims};

/// Extension type to store authenticated user claims
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub email: String,
    pub is_admin: bool,
}

impl From<Claims> for AuthUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            email: claims.email,
            is_admin: claims.is_admin,
        }
    }
}

/// JWT authentication middleware
pub async fn auth_middleware(
    State(jwt_secret): State<String>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    // Check if it starts with "Bearer "
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    // Verify the token
    let claims = verify_token(token, &jwt_secret).map_err(|_| AuthError::InvalidToken)?;

    // Add user info to request extensions
    let auth_user = AuthUser::from(claims);
    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Admin-only authentication middleware
pub async fn admin_middleware(
    State(jwt_secret): State<String>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    // Check if it starts with "Bearer "
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    // Verify the token
    let claims = verify_token(token, &jwt_secret).map_err(|_| AuthError::InvalidToken)?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(AuthError::Forbidden);
    }

    // Add user info to request extensions
    let auth_user = AuthUser::from(claims);
    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    Forbidden,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "MISSING_TOKEN",
                "Authorization token is missing",
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Authorization token is invalid or expired",
            ),
            AuthError::Forbidden => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                "You don't have permission to access this resource",
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

/// Rate limiting middleware - limits requests per user per minute
/// Uses Redis to track request counts
pub async fn rate_limit_middleware(
    State((redis_conn, jwt_secret)): State<(Arc<ConnectionManager>, String)>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // Determine rate limit key based on authentication
    let rate_limit_key = if let Some(auth_str) = auth_header {
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            // Try to verify token and use user_id as key
            if let Ok(claims) = verify_token(token, &jwt_secret) {
                format!("rate_limit:user:{}", claims.sub)
            } else {
                // Invalid token, use IP-based rate limiting (if available)
                // For now, use a generic key
                "rate_limit:anonymous".to_string()
            }
        } else {
            "rate_limit:anonymous".to_string()
        }
    } else {
        "rate_limit:anonymous".to_string()
    };

    // Check rate limit using Redis
    let mut conn = redis_conn.as_ref().clone();
    
    // Use Redis INCR with expiration
    let count: Result<i64, redis::RedisError> = redis::cmd("INCR")
        .arg(&rate_limit_key)
        .query_async(&mut conn)
        .await;

    match count {
        Ok(current_count) => {
            // Set expiration on first request (count == 1)
            if current_count == 1 {
                let _: Result<(), redis::RedisError> = redis::cmd("EXPIRE")
                    .arg(&rate_limit_key)
                    .arg(60) // 60 seconds
                    .query_async(&mut conn)
                    .await;
            }

            // Check if rate limit exceeded (60 requests per minute)
            if current_count > 60 {
                return Err(RateLimitError::TooManyRequests);
            }

            // Add rate limit headers to response
            let mut response = next.run(request).await;
            let headers = response.headers_mut();
            headers.insert("X-RateLimit-Limit", "60".parse().unwrap());
            headers.insert("X-RateLimit-Remaining", (60 - current_count).max(0).to_string().parse().unwrap());
            
            Ok(response)
        }
        Err(e) => {
            // If Redis fails, log error and allow request (fail open)
            tracing::warn!("Rate limit check failed: {}. Allowing request.", e);
            Ok(next.run(request).await)
        }
    }
}

/// Rate limiting errors
#[derive(Debug)]
pub enum RateLimitError {
    TooManyRequests,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            RateLimitError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Too many requests. Please try again later.",
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_from_claims() {
        let claims = Claims {
            sub: 123,
            email: "test@example.com".to_string(),
            is_admin: true,
            exp: 1234567890,
            iat: 1234567800,
        };

        let auth_user = AuthUser::from(claims);
        
        assert_eq!(auth_user.user_id, 123);
        assert_eq!(auth_user.email, "test@example.com");
        assert!(auth_user.is_admin);
    }

    #[test]
    fn test_auth_error_responses() {
        let missing_token_response = AuthError::MissingToken.into_response();
        assert_eq!(missing_token_response.status(), StatusCode::UNAUTHORIZED);

        let invalid_token_response = AuthError::InvalidToken.into_response();
        assert_eq!(invalid_token_response.status(), StatusCode::UNAUTHORIZED);

        let forbidden_response = AuthError::Forbidden.into_response();
        assert_eq!(forbidden_response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_rate_limit_error_response() {
        let error = RateLimitError::TooManyRequests;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
