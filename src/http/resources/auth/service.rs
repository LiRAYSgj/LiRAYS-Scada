use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use time::Duration as CookieDuration;

use crate::http::resources::static_files::service::serve_static_path;
use crate::http::resources::user::service::UserCredentials;
use crate::http::{
    ApiResponse, ApiResponseEmpty, ApiTokenResponse, AppState, TokenType, issue_token,
    refresh_cookie, session_cookie, token_is_valid,
};

pub async fn setup_get(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }

    let admin_exists = state.users.admin_exists().await.unwrap_or(false);
    if admin_exists {
        let token = jar
            .get("lirays_session")
            .map(|cookie| cookie.value().to_string());
        if let Some(token) = token {
            if token_is_valid(&state.auth, &token, TokenType::Access) {
                return Redirect::to("/").into_response();
            }
        }

        return Redirect::to("/auth/login").into_response();
    }

    serve_static_path("/auth/setup").into_response()
}

#[derive(Deserialize)]
pub struct PasswordForm {
    password: String,
}

pub async fn setup_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::extract::Form(form): axum::extract::Form<PasswordForm>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponseEmpty::error(
                "Authentication disabled".to_string(),
            )),
        )
            .into_response();
    }
    if state.users.admin_exists().await.unwrap_or(true) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponseEmpty::error("Admin user already exists".to_string())),
        )
            .into_response();
    }
    if form.password.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponseEmpty::error("Password required".to_string())),
        )
            .into_response();
    }
    if let Err(_) = state.users.create_admin(form.password).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(
                "Failed to create admin user".to_string(),
            )),
        )
            .into_response();
    }

    let (access_token, _) =
        issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
    let (refresh_token, _) =
        issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
    let jar = jar
        .add(session_cookie(
            access_token,
            state.auth.secure_cookies,
            state.auth.access_ttl,
        ))
        .add(refresh_cookie(
            refresh_token,
            state.auth.secure_cookies,
            state.auth.refresh_ttl,
        ));
    let response = (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response();
    (jar, response).into_response()
}

pub async fn login_get(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }

    let admin_exists = state.users.admin_exists().await.unwrap_or(false);
    if !admin_exists {
        return Redirect::to("/auth/setup").into_response();
    }

    let token = jar
        .get("lirays_session")
        .map(|cookie| cookie.value().to_string());
    if let Some(token) = token {
        if token_is_valid(&state.auth, &token, TokenType::Access) {
            return Redirect::to("/").into_response();
        }
    }

    serve_static_path("/auth/login").into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatusResponse {
    auth_enabled: bool,
    authenticated: bool,
    admin_exists: bool,
}

pub async fn auth_status(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return Json(AuthStatusResponse {
            auth_enabled: false,
            authenticated: false,
            admin_exists: false,
        });
    }
    let admin_exists = state.users.admin_exists().await.unwrap_or(false);

    let token = jar
        .get("lirays_session")
        .map(|cookie| cookie.value().to_string());
    let authenticated = token
        .map(|token| token_is_valid(&state.auth, &token, TokenType::Access))
        .unwrap_or(false);

    Json(AuthStatusResponse {
        auth_enabled: true,
        authenticated,
        admin_exists,
    })
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::extract::Form(form): axum::extract::Form<LoginForm>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponseEmpty::error(
                "Authentication disabled".to_string(),
            )),
        )
            .into_response();
    }
    let creds = UserCredentials {
        username: form.username,
        password: form.password,
    };
    match state.users.verify(&creds).await {
        Ok(true) => {
            let (access_token, _) =
                issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
            let (refresh_token, _) =
                issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
            let jar = jar
                .add(session_cookie(
                    access_token,
                    state.auth.secure_cookies,
                    state.auth.access_ttl,
                ))
                .add(refresh_cookie(
                    refresh_token,
                    state.auth.secure_cookies,
                    state.auth.refresh_ttl,
                ));
            let response = (
                StatusCode::OK,
                Json(ApiResponseEmpty::success()),
            )
                .into_response();
            (jar, response).into_response()
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponseEmpty::error("Invalid credentials.".to_string())),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error("Unable to log in right now. Please try again.".to_string())),
        )
            .into_response(),
    }
}

pub async fn login_api(
    State(state): State<Arc<AppState>>,
    Json(creds): Json<UserCredentials>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponseEmpty::error(
                "Authentication disabled".to_string(),
            )),
        )
            .into_response();
    }

    match state.users.verify(&creds).await {
        Ok(true) => {
            let (token, exp) =
                issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
            let (refresh_token, refresh_exp) =
                issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
            (
                StatusCode::OK,
                Json(ApiResponse::success(ApiTokenResponse {
                    token,
                    refresh_token,
                    expires_at: exp,
                    refresh_expires_at: refresh_exp,
                })),
            )
                .into_response()
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponseEmpty::error("Invalid credentials".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {e}"))),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: Option<String>,
}

pub async fn refresh_api(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(body): Json<RefreshRequest>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponseEmpty::error(
                "Authentication disabled".to_string(),
            )),
        )
            .into_response();
    }

    let token = body
        .refresh_token
        .or_else(|| jar.get("lirays_refresh").map(|c| c.value().to_string()));

    let Some(refresh_token) = token else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponseEmpty::error("Refresh token missing".to_string())),
        )
            .into_response();
    };

    if !token_is_valid(&state.auth, &refresh_token, TokenType::Refresh) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponseEmpty::error(
                "Invalid or expired refresh token".to_string(),
            )),
        )
            .into_response();
    }

    let (access_token, exp) =
        issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
    let (new_refresh_token, refresh_exp) =
        issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();

    let response = (
        StatusCode::OK,
        Json(ApiResponse::success(ApiTokenResponse {
            token: access_token.clone(),
            refresh_token: new_refresh_token.clone(),
            expires_at: exp,
            refresh_expires_at: refresh_exp,
        })),
    )
        .into_response();

    let jar = jar
        .add(session_cookie(
            access_token,
            state.auth.secure_cookies,
            state.auth.access_ttl,
        ))
        .add(refresh_cookie(
            new_refresh_token,
            state.auth.secure_cookies,
            state.auth.refresh_ttl,
        ));
    (jar, response).into_response()
}

pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }
    let mut cookie = Cookie::build(("lirays_session", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .build();
    if state.auth.secure_cookies {
        cookie.set_secure(true);
    }
    let mut refresh = Cookie::build(("lirays_refresh", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .build();
    if state.auth.secure_cookies {
        refresh.set_secure(true);
    }
    let jar = jar.add(cookie).add(refresh);
    (jar, Redirect::to("/auth/login")).into_response()
}
