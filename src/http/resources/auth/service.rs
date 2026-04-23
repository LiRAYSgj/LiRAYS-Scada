use std::sync::Arc;

use axum::{
    Json,
    extract::{Form, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration as CookieDuration;

use super::namespace::{AuthStatusResponse, LoginForm, PasswordForm, UserInfo};
use crate::http::resources::user::model::Role;
use crate::http::resources::user::namespace::UserCredentials;
use crate::http::{ApiResponseEmpty, AppState};

pub fn session_cookie(
    session_id: String,
    secure: bool,
    exp_time_sec: Option<i64>,
) -> Cookie<'static> {
    let cookie_builder = Cookie::build(("lirays_session", session_id))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax);
    let mut cookie = match exp_time_sec {
        Some(tsec) => cookie_builder
            .max_age(CookieDuration::seconds(tsec))
            .build(),
        None => cookie_builder.build(),
    };
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

/// Bootstrap endpoint to create the first admin account and log it in.
pub async fn setup_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<PasswordForm>,
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
            Json(ApiResponseEmpty::error(
                "Admin user already exists".to_string(),
            )),
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

    let session_id = match state.session_manager.create_session("admin", Role::Admin) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponseEmpty::error(format!(
                    "Unable to create session: {e}"
                ))),
            )
                .into_response();
        }
    };

    let jar = jar.add(session_cookie(session_id, state.auth.secure_cookies, None));
    let response = (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response();
    (jar, response).into_response()
}

/// Login endpoint using username/password and session cookie auth.
pub async fn login_session_id_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
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
    match state.users.get_user(&creds.username).await {
        Ok(Some(user)) => {
            if !state
                .users
                .verify_password(&user, &creds.password)
                .await
                .unwrap_or(false)
            {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponseEmpty::error("Invalid credentials.".to_string())),
                )
                    .into_response();
            }
            let session_id = match state
                .session_manager
                .create_session(&creds.username, user.role)
            {
                Ok(id) => id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponseEmpty::error(format!(
                            "Unable to create session: {e}"
                        ))),
                    )
                        .into_response();
                }
            };

            let jar = jar.add(session_cookie(session_id, state.auth.secure_cookies, None));
            let response = (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response();
            (jar, response).into_response()
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponseEmpty::error(
                "The user has not been set up yet.".to_string(),
            )),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(
                "Unable to log in right now. Please try again.".to_string(),
            )),
        )
            .into_response(),
    }
}

/// Return auth state for UI bootstrap flow.
pub async fn auth_status(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return Json(AuthStatusResponse {
            auth_enabled: false,
            authenticated: false,
            admin_exists: false,
            user: None,
        });
    }
    let admin_exists = state.users.admin_exists().await.unwrap_or(false);
    if !admin_exists {
        return Json(AuthStatusResponse {
            auth_enabled: true,
            authenticated: false,
            admin_exists,
            user: None,
        });
    }

    let mut user_info = None;
    let mut authenticated = false;

    if let Some(cookie) = jar.get("lirays_session") {
        let session_id = cookie.value();
        if state.session_manager.verify_session_id(session_id) {
            authenticated = true;
            if let Some(session) = state.session_manager.load_session(session_id) {
                if let Ok(Some(user)) = state.users.get_user(&session.username).await {
                    user_info = Some(UserInfo {
                        username: user.username,
                        role: user.role,
                    });
                }
            }
        }
    }
    match user_info {
        Some(u_info) => Json(AuthStatusResponse {
            auth_enabled: true,
            authenticated,
            admin_exists,
            user: Some(u_info),
        }),
        None => Json(AuthStatusResponse {
            auth_enabled: true,
            authenticated: false,
            admin_exists,
            user: None,
        }),
    }
}

/// Logout endpoint that revokes the current session cookie.
pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if !state.auth.enabled {
        return (jar, Json(ApiResponseEmpty::success())).into_response();
    }
    let session_id = jar
        .get("lirays_session")
        .map(|cookie| cookie.value())
        .unwrap_or("");
    state.session_manager.revoke_session(session_id, false);
    let jar = jar.add(session_cookie(
        "".to_string(),
        state.auth.secure_cookies,
        Some(0),
    ));
    (jar, Json(ApiResponseEmpty::success())).into_response()
}
