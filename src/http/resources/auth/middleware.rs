use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;

use super::service::session_cookie;
use crate::http::resources::user::model::Role;
use crate::http::{AppState, AuthContext};

fn bearer_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()))
}

/// Resolve caller role from PAT bearer token or session cookie.
///
/// Returns an updated cookie jar when session rotation occurs.
async fn _extract_role(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    bearer_token: Option<String>,
) -> Result<(Role, Option<CookieJar>), StatusCode> {
    match bearer_token {
        Some(token) => match state.patoken_manager.verify(&token).await {
            Ok(role) => Ok((role, None)),
            Err(_) => Err(StatusCode::UNAUTHORIZED),
        },
        None => match jar.get("lirays_session") {
            Some(cookie) => {
                let session_id = cookie.value();
                match state.session_manager.session_claims(session_id) {
                    Some(token) => Ok((token.role, None)),
                    None => match state.session_manager.refresh_session(session_id) {
                        Ok(new_session_id) => {
                            match state.session_manager.session_claims(&new_session_id) {
                                Some(token) => {
                                    let cookie = session_cookie(
                                        new_session_id,
                                        state.auth.secure_cookies,
                                        None,
                                    );
                                    Ok((token.role, Some(jar.add(cookie))))
                                }
                                None => Err(StatusCode::UNAUTHORIZED),
                            }
                        }
                        Err(_) => Err(StatusCode::UNAUTHORIZED),
                    },
                }
            }
            None => Err(StatusCode::UNAUTHORIZED),
        },
    }
}

/// Middleware that enforces authentication for routes that require any role.
///
/// When a valid session is refreshed, the new cookie is attached to the response.
pub async fn authenticated_only(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let jar = CookieJar::from_headers(req.headers());
    let bearer_token = bearer_from_headers(req.headers());
    if state.auth.enabled {
        let path = req.uri().path();
        let (role, opt_jar) = match _extract_role(State(state), jar, bearer_token).await {
            Ok(value) => value,
            Err(status) => return status.into_response(),
        };
        if path.starts_with("/ws") {
            req.extensions_mut().insert(AuthContext { role });
        }
        match opt_jar {
            Some(jar_) => (jar_, next.run(req).await).into_response(),
            None => next.run(req).await,
        }
    } else {
        next.run(req).await
    }
}

/// Middleware that enforces admin-only access.
pub async fn admin_only(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let jar = CookieJar::from_headers(req.headers());
    let bearer_token = bearer_from_headers(req.headers());
    if state.auth.enabled {
        let (role, opt_jar) = match _extract_role(State(state), jar, bearer_token).await {
            Ok(value) => value,
            Err(status) => return status.into_response(),
        };
        match role {
            Role::Admin => match opt_jar {
                Some(jar_) => (jar_, (next.run(req).await)).into_response(),
                None => next.run(req).await,
            },
            _ => StatusCode::UNAUTHORIZED.into_response(),
        }
    } else {
        next.run(req).await
    }
}
