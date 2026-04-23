pub mod session;
pub mod token;

use std::{sync::Arc, time::Duration};

use log::{info, warn};
use session::{Session, decode_token_claims, is_expired_error};
use sled::Tree;
use token::TokenClaims;
use tokio::{task, time};
use uuid::Uuid;

use crate::http::resources::user::model::Role;

const SESSION_KEY_PREFIX: &str = "user_session";
const SESSION_ACCESS_KEY_PREFIX: &str = "user_session_access";
const PURGE_INTERVAL_SECS: u64 = 60 * 60 * 12;

/// Persists login sessions in a sled tree and manages refresh/revocation lifecycle.
pub struct SessionManager {
    pub sessions_tree: Tree,
    access_secret: String,
    refresh_secret: String,
    access_ttl_secs: u64,
    refresh_ttl_secs: u64,
}

impl SessionManager {
    /// Open/create the session store in `files_dir`.
    ///
    /// Returns an error instead of panicking if the DB path/tree cannot be created.
    pub fn new(
        files_dir: &str,
        access_secret: &str,
        refresh_secret: &str,
        access_ttl_secs: u64,
        refresh_ttl_secs: u64,
    ) -> Result<Self, String> {
        let db = sled::open(files_dir)
            .map_err(|e| format!("Failed to open session DB store at '{files_dir}': {e}"))?;
        let sessions_tree = db
            .open_tree("sessionTree")
            .map_err(|e| format!("Failed to open session tree: {e}"))?;

        Ok(Self {
            sessions_tree,
            access_secret: access_secret.to_string(),
            refresh_secret: refresh_secret.to_string(),
            access_ttl_secs,
            refresh_ttl_secs,
        })
    }

    /// Persist a session payload and return the generated session id.
    pub fn save_session(&self, session: Session) -> Result<String, String> {
        let session_id = Uuid::new_v4().to_string();
        let user_name = session.username.clone();
        let session_data_key = format!("{SESSION_KEY_PREFIX}:{session_id}");
        let session_uname_key = format!("{SESSION_ACCESS_KEY_PREFIX}:{user_name}:{session_id}");

        self.sessions_tree
            .insert(
                session_data_key,
                serde_json::to_vec(&session)
                    .map_err(|e| format!("Error serializing session: {e}"))?,
            )
            .map_err(|e| format!("Error inserting session: {e}"))?;
        self.sessions_tree
            .insert(session_uname_key, "")
            .map_err(|e| format!("Error inserting session: {e}"))?;

        Ok(session_id)
    }

    /// Load a session by id. Returns `None` for missing/corrupt entries.
    pub fn load_session(&self, session_id: &str) -> Option<Session> {
        let session_data_key = format!("{SESSION_KEY_PREFIX}:{session_id}");
        match self.sessions_tree.get(session_data_key) {
            Ok(Some(data)) => serde_json::from_slice::<Session>(&data).ok(),
            _ => None,
        }
    }

    /// List active session ids for a user.
    pub fn get_user_sessions(&self, user_name: &str) -> Vec<String> {
        let prefix = format!("{SESSION_ACCESS_KEY_PREFIX}:{user_name}:");

        self.sessions_tree
            .scan_prefix(prefix.as_bytes())
            .filter_map(|res| {
                res.ok().and_then(|(key, _)| {
                    std::str::from_utf8(&key)
                        .ok()
                        .and_then(|k| k.rsplit(':').next())
                        .map(|id| id.to_string())
                })
            })
            .collect()
    }

    fn remove_session(&self, username: &str, session_id: &str) {
        let session_data_key = format!("{SESSION_KEY_PREFIX}:{session_id}");
        let session_uname_key = format!("{SESSION_ACCESS_KEY_PREFIX}:{username}:{session_id}");

        match self.sessions_tree.remove(session_data_key) {
            Err(e) => warn!("Error removing session: {}", e),
            _ => (),
        }
        match self.sessions_tree.remove(session_uname_key) {
            Err(e) => warn!("Error removing session: {}", e),
            _ => (),
        }
    }

    /// Create and persist a new signed session for a user.
    pub fn create_session(&self, username: &str, role: Role) -> Result<String, String> {
        let session = Session::new(
            username,
            self.access_secret.as_bytes(),
            self.refresh_secret.as_bytes(),
            self.access_ttl_secs,
            self.refresh_ttl_secs,
            role,
        )?;
        self.save_session(session)
    }

    /// Revoke one session or all sessions of the same user (`full = true`).
    pub fn revoke_session(&self, session_id: &str, full: bool) {
        let Some(session) = self.load_session(session_id) else {
            // Already removed or never existed.
            return;
        };

        let username = session.username;

        if full {
            for id in self.get_user_sessions(&username) {
                self.remove_session(&username, &id);
            }
        } else {
            self.remove_session(&username, session_id);
        }
    }

    /// Rotate a session using its refresh token and return the new session id.
    pub fn refresh_session(&self, session_id: &str) -> Result<String, String> {
        let session = self
            .load_session(session_id)
            .ok_or("Session not found or expired".to_string())?;

        let claims = decode_token_claims(&session.refresh_token, self.refresh_secret.as_bytes())
            .map_err(|e| format!("Invalid refresh token: {e}"))?;

        if claims.token_type != "refresh" {
            return Err("Invalid token type".to_string());
        }
        // revoke current session
        self.revoke_session(session_id, false);

        // create new session for the same user
        self.create_session(&claims.user, claims.role)
    }

    /// Validate the access token behind a session id.
    pub fn verify_session_id(&self, session_id: &str) -> bool {
        if let Some(session) = self.load_session(session_id) {
            if let Ok(claims) =
                decode_token_claims(&session.access_token, self.access_secret.as_bytes())
            {
                return claims.token_type == "access";
            }
        }
        false
    }

    /// Decode access-token claims from a stored session id.
    pub fn session_claims(&self, session_id: &str) -> Option<TokenClaims> {
        let session = self.load_session(session_id)?;
        decode_token_claims(&session.access_token, self.access_secret.as_bytes()).ok()
    }

    /// Scan stored sessions and revoke entries with expired refresh tokens.
    pub fn purge_expired_sessions(&self) {
        // Iterate over all session data keys.
        for entry in self
            .sessions_tree
            .scan_prefix(SESSION_KEY_PREFIX.as_bytes())
        {
            let Ok((key, value)) = entry else {
                continue;
            };

            let Ok(key_str) = std::str::from_utf8(&key) else {
                continue;
            };

            // Expect keys shaped like "user_session:{session_id}"
            let Some(session_id) = key_str.split(':').nth(1) else {
                continue;
            };

            // Attempt to deserialize the session and decode its refresh token for exp.
            let Ok(session) = serde_json::from_slice::<Session>(&value) else {
                continue;
            };

            match decode_token_claims(&session.refresh_token, self.refresh_secret.as_bytes()) {
                Ok(_) => {}
                Err(err) if is_expired_error(&err) => {
                    info!("Revoking session: {}", session_id);
                    self.revoke_session(session_id, false);
                }
                Err(_) => {}
            }
        }
    }

    /// Spawn a background loop that purges expired sessions every 12 hours.
    /// The purge itself runs in `spawn_blocking` to avoid blocking async executors
    /// while sled performs disk I/O.
    pub fn start_purge_loop(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(PURGE_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let manager = self.clone();
                if let Err(err) =
                    task::spawn_blocking(move || manager.purge_expired_sessions()).await
                {
                    warn!("purge_expired_sessions panicked: {err}");
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use super::*;

    fn temp_sled_dir(prefix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("{}_{}", prefix, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn create_and_verify_session_roundtrip() {
        let dir = temp_sled_dir("session_test_roundtrip");
        let manager =
            SessionManager::new(dir.to_str().expect("utf8 path"), "acc", "ref", 60, 60).unwrap();

        let session_id = manager.create_session("alice", Role::Admin).unwrap();
        assert!(manager.verify_session_id(&session_id));

        let claims = manager
            .session_claims(&session_id)
            .expect("claims for valid session");
        assert_eq!(claims.user, "alice");
        assert_eq!(claims.role, Role::Admin);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn refresh_session_rotates_and_revokes_old_session() {
        let dir = temp_sled_dir("session_test_refresh");
        let manager =
            SessionManager::new(dir.to_str().expect("utf8 path"), "acc", "ref", 60, 60).unwrap();

        let old_id = manager.create_session("operator1", Role::Operator).unwrap();
        let new_id = manager.refresh_session(&old_id).unwrap();

        assert_ne!(old_id, new_id);
        assert!(!manager.verify_session_id(&old_id));
        assert!(manager.verify_session_id(&new_id));

        let sessions = manager.get_user_sessions("operator1");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], new_id);
    }

    #[test]
    fn revoke_full_removes_all_user_sessions() {
        let dir = temp_sled_dir("session_test_revoke_full");
        let manager =
            SessionManager::new(dir.to_str().expect("utf8 path"), "acc", "ref", 60, 60).unwrap();

        let s1 = manager.create_session("alice", Role::Operator).unwrap();
        let s2 = manager.create_session("alice", Role::Operator).unwrap();
        let other = manager.create_session("bob", Role::Admin).unwrap();

        manager.revoke_session(&s1, true);

        assert!(manager.load_session(&s1).is_none());
        assert!(manager.load_session(&s2).is_none());
        assert!(manager.load_session(&other).is_some());
    }

    #[test]
    fn purge_expired_sessions_removes_entries_with_expired_refresh_token() {
        let dir = temp_sled_dir("session_test_purge");
        let access_secret = "access_secret";
        let refresh_secret = "refresh_secret";
        let manager = SessionManager::new(
            dir.to_str().expect("utf8 path"),
            access_secret,
            refresh_secret,
            60,
            60,
        )
        .unwrap();

        let now = chrono::Utc::now();
        let access_claims = token::TokenClaims {
            user: "eve".to_string(),
            token_type: "access".to_string(),
            role: Role::Operator,
            exp: (now + chrono::Duration::seconds(60)).timestamp(),
            issued_at: now.timestamp_millis(),
        };
        let refresh_claims = token::TokenClaims {
            user: "eve".to_string(),
            token_type: "refresh".to_string(),
            role: Role::Operator,
            exp: (now - chrono::Duration::seconds(3600)).timestamp(),
            issued_at: now.timestamp_millis(),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(access_secret.as_bytes()),
        )
        .unwrap();
        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(refresh_secret.as_bytes()),
        )
        .unwrap();

        let session_id = manager
            .save_session(Session {
                access_token,
                refresh_token,
                username: "eve".to_string(),
            })
            .unwrap();
        assert!(manager.load_session(&session_id).is_some());

        manager.purge_expired_sessions();
        assert!(manager.load_session(&session_id).is_none());
    }
}
