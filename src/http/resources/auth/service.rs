use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use time::Duration as CookieDuration;

use crate::http::resources::user::service::UserCredentials;
use crate::http::{
    ApiResponse, ApiResponseEmpty, ApiTokenResponse, AppState, TokenType, issue_token, refresh_cookie,
    session_cookie, token_is_valid,
};

pub async fn setup_get() -> impl IntoResponse {
    const FORM: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Set admin password</title>
  <style>
    :root { color-scheme: light dark; }
    body {
      margin:0; padding:0;
      font-family: -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
      background: radial-gradient(circle at 20% 20%, #1e3a8a22, transparent 35%),
                  radial-gradient(circle at 80% 0%, #0ea5e922, transparent 40%),
                  #0b1220;
      color: #e5e7eb;
      display:flex; align-items:center; justify-content:center; min-height:100vh;
    }
    .card {
      background: #0f172a;
      border: 1px solid #1f2937;
      padding: 28px;
      border-radius: 14px;
      width: min(420px, 90vw);
      box-shadow: 0 20px 50px rgba(0,0,0,0.45);
    }
    h1 { margin: 0 0 8px; font-size: 24px; }
    p { margin: 0 0 18px; color: #9ca3af; line-height: 1.5; }
    label { display:block; margin:14px 0 6px; font-weight:600; }
    input {
      width: 100%; padding: 12px 14px;
      border-radius: 10px; border: 1px solid #1f2937;
      background: #111827; color: #e5e7eb;
      font-size: 15px;
    }
    button {
      margin-top: 18px; width: 100%;
      padding: 12px 16px;
      border: none; border-radius: 10px;
      background: linear-gradient(135deg, #2563eb, #0ea5e9);
      color: white; font-weight: 700; font-size: 15px;
      cursor: pointer; transition: transform 120ms ease, filter 120ms ease;
    }
    button:hover { transform: translateY(-1px); filter: brightness(1.05); }
    .note { font-size: 13px; color: #9ca3af; margin-top: 10px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Set admin password</h1>
    <p>First-time setup: define the password for user <strong>admin</strong>.</p>
    <form method="post" action="/auth/setup">
      <label for="password">New password</label>
      <input id="password" name="password" type="password" required minlength="6" autofocus />
      <button type="submit">Save and continue</button>
      <div class="note">Stored server-side as an Argon2 hash.</div>
    </form>
  </div>
</body>
</html>"#;
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(FORM))
        .unwrap()
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
        return Redirect::to("/").into_response();
    }
    if state.users.admin_exists().await.unwrap_or(true) {
        return Redirect::to("/auth/login").into_response();
    }
    if form.password.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Password required").into_response();
    }
    if let Err(e) = state.users.create_admin(form.password).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error creando admin: {e}"),
        )
            .into_response();
    }

    let (access_token, _) = issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
    let (refresh_token, _) = issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
    let jar = jar
        .add(session_cookie(access_token, state.auth.secure_cookies, state.auth.access_ttl))
        .add(refresh_cookie(refresh_token, state.auth.secure_cookies, state.auth.refresh_ttl));
    (jar, Redirect::to("/")).into_response()
}

pub async fn login_get() -> impl IntoResponse {
    const FORM: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Login</title>
  <style>
    :root { color-scheme: light dark; }
    body {
      margin:0; padding:0;
      font-family: -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
      background: radial-gradient(circle at 10% 10%, #16a34a22, transparent 35%),
                  radial-gradient(circle at 80% 20%, #2563eb22, transparent 40%),
                  #0b1220;
      color: #e5e7eb;
      display:flex; align-items:center; justify-content:center; min-height:100vh;
    }
    .card {
      background: #0f172a;
      border: 1px solid #1f2937;
      padding: 28px;
      border-radius: 14px;
      width: min(420px, 90vw);
      box-shadow: 0 20px 50px rgba(0,0,0,0.45);
    }
    h1 { margin: 0 0 8px; font-size: 24px; }
    p { margin: 0 0 18px; color: #9ca3af; line-height: 1.5; }
    label { display:block; margin:14px 0 6px; font-weight:600; }
    input {
      width: 100%; padding: 12px 14px;
      border-radius: 10px; border: 1px solid #1f2937;
      background: #111827; color: #e5e7eb;
      font-size: 15px;
    }
    button {
      margin-top: 18px; width: 100%;
      padding: 12px 16px;
      border: none; border-radius: 10px;
      background: linear-gradient(135deg, #16a34a, #22c55e);
      color: white; font-weight: 700; font-size: 15px;
      cursor: pointer; transition: transform 120ms ease, filter 120ms ease;
    }
    button:hover { transform: translateY(-1px); filter: brightness(1.05); }
    .hint { font-size: 13px; color: #9ca3af; margin-top: 10px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Sign in</h1>
    <p>Use the <strong>admin</strong> account.</p>
    <form method="post" action="/auth/login">
      <label for="username">Username</label>
      <input id="username" name="username" value="admin" required />
      <label for="password">Password</label>
      <input id="password" name="password" type="password" required />
      <button type="submit">Sign in</button>
      <div class="hint">If admin doesn’t exist yet, go to /auth/setup first.</div>
    </form>
  </div>
</body>
</html>"#;
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(FORM))
        .unwrap()
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
        return Redirect::to("/").into_response();
    }
    let creds = UserCredentials {
        username: form.username,
        password: form.password,
    };
    match state.users.verify(&creds).await {
        Ok(true) => {
            let (access_token, _) = issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
            let (refresh_token, _) = issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
            let jar = jar
                .add(session_cookie(access_token, state.auth.secure_cookies, state.auth.access_ttl))
                .add(refresh_cookie(refresh_token, state.auth.secure_cookies, state.auth.refresh_ttl));
            (jar, Redirect::to("/")).into_response()
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            "Credenciales inválidas",
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error de base de datos: {e}"),
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
            Json(ApiResponseEmpty::error("Authentication disabled".to_string())),
        )
            .into_response();
    }

    match state.users.verify(&creds).await {
        Ok(true) => {
            let (token, exp) = issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
            let (refresh_token, refresh_exp) = issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();
            (StatusCode::OK, Json(ApiResponse::success(ApiTokenResponse {
                token,
                refresh_token,
                expires_at: exp,
                refresh_expires_at: refresh_exp,
            }))).into_response()
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
            Json(ApiResponseEmpty::error("Authentication disabled".to_string())),
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
            Json(ApiResponseEmpty::error("Invalid or expired refresh token".to_string())),
        )
            .into_response();
    }

    let (access_token, exp) = issue_token(&state.auth, 1, TokenType::Access, state.auth.access_ttl).unwrap();
    let (new_refresh_token, refresh_exp) = issue_token(&state.auth, 1, TokenType::Refresh, state.auth.refresh_ttl).unwrap();

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

pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> impl IntoResponse {
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
