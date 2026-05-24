use crate::{ApiError, ApiResult, AppState};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

// ---------------------------------------------------------------------------
// JWT claims
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

// ---------------------------------------------------------------------------
// Request / Response DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// ---------------------------------------------------------------------------
// Login handler
// ---------------------------------------------------------------------------

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    if body.username != "admin" {
        return Err(ApiError::unauthorized("invalid credentials"));
    }

    if body.password != state.config.auth.admin_password {
        return Err(ApiError::unauthorized("invalid credentials"));
    }

    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(state.config.auth.token_expiry_hours as i64);

    let claims = Claims {
        sub: body.username,
        iat: now.unix_timestamp(),
        exp: exp.unix_timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.auth.jwt_secret.as_bytes()),
    )
    .map_err(ApiError::from)?;

    Ok(Json(LoginResponse { token }))
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "missing or invalid authorization header"})),
            )
                .into_response();
        }
    };

    let validation = Validation::default();
    let result = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.auth.jwt_secret.as_bytes()),
        &validation,
    );

    match result {
        Ok(token_data) => {
            req.extensions_mut().insert(token_data.claims);
            next.run(req).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid token"})),
        )
            .into_response(),
    }
}
