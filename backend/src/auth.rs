//! 密码鉴权模块
//!
//! 提供密码哈希、session token 生成与校验、以及一个全局 axum 中间件。
//! Session 只存内存，不落库，设备重启即失效。

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::RngCore;

use crate::models::ApiResponse;
use crate::state::AppState;

pub const SESSION_COOKIE_NAME: &str = "udx710_session";
const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 无需鉴权即可访问的路径前缀
const PUBLIC_PATHS: &[&str] = &["/api/auth/login", "/api/auth/status", "/api/health"];

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Failed to hash password: {}", e))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 内存 session 存储，token -> 过期时间
pub struct SessionStore {
    sessions: RwLock<HashMap<String, Instant>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn create(&self) -> String {
        let token = generate_session_token();
        self.sessions
            .write()
            .unwrap()
            .insert(token.clone(), Instant::now() + SESSION_TTL);
        token
    }

    pub fn validate(&self, token: &str) -> bool {
        self.sessions
            .read()
            .unwrap()
            .get(token)
            .is_some_and(|expires_at| *expires_at > Instant::now())
    }

    pub fn remove(&self, token: &str) {
        self.sessions.write().unwrap().remove(token);
    }

    /// 踢掉所有会话（修改密码时使用）
    pub fn remove_all(&self) {
        self.sessions.write().unwrap().clear();
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.sessions.write().unwrap().retain(|_, expires_at| *expires_at > now);
    }
}

/// 从请求头解析出 session token
pub fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        let (name, value) = part.split_once('=')?;
        if name == SESSION_COOKIE_NAME {
            Some(value.to_string())
        } else {
            None
        }
    })
}

pub fn build_set_cookie_header(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        SESSION_COOKIE_NAME,
        token,
        SESSION_TTL.as_secs()
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

pub fn build_clear_cookie_header() -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        SESSION_COOKIE_NAME
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// 全局鉴权中间件：未开启鉴权时全部放行；开启后校验 session cookie
///
/// 只拦截 `/api/*` 路径。SPA 静态资源（index.html、JS/CSS 包）永远放行——
/// 否则鉴权出现任何异常时，连登录页本身都加载不出来，会造成彻底无法访问。
pub async fn auth_middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();

    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    if PUBLIC_PATHS.iter().any(|p| path == *p) {
        return next.run(req).await;
    }

    let auth_config = state.config_manager.get_auth();
    if !auth_config.enabled {
        return next.run(req).await;
    }

    let authorized = extract_session_token(req.headers())
        .is_some_and(|token| state.session_store.validate(&token));

    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("未登录或会话已过期")),
        )
            .into_response();
    }

    next.run(req).await
}
