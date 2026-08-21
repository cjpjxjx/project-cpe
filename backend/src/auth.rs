//! 密码鉴权模块
//!
//! 提供密码哈希、session token 生成与校验、以及一个全局 axum 中间件。
//! Session 只存内存，不落库，设备重启即失效。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use crate::models::ApiResponse;
use crate::state::AppState;

pub const SESSION_COOKIE_NAME: &str = "udx710_session";
const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 无需鉴权即可访问的路径前缀
const PUBLIC_PATHS: &[&str] = &["/api/auth/login", "/api/auth/status", "/api/health"];

/// 新密码哈希使用的参数：内存 8 MiB、1 轮迭代。
///
/// Argon2 的默认参数（19 MiB / 2 轮）是 RFC 9106 面向互联网服务的推荐值，针对
/// 的是「哈希库被拖库后攻击者用 GPU 集群离线爆破」这种场景。这里保护的是单台
/// 本地设备的管理密码，攻击者必须先接触到设备所在网络才能碰到登录接口，因此
/// 调轻参数：UDX710 需要与 ofonod/sprdrild 等原厂进程抢 CPU，默认参数下单次
/// 哈希/校验可能耗时数十秒。加盐哈希在更低参数下仍远强于无盐/快速哈希。
///
/// 仅影响新生成的哈希：`verify_password` 校验时使用的是哈希串里内嵌的历史
/// 参数（见 `password-hash` crate 的 `PasswordVerifier` blanket 实现），不受
/// 这里改动影响，旧密码要重新设置一次才会换成轻量参数。
fn new_password_params() -> Params {
    Params::new(8 * 1024, 1, Params::DEFAULT_P_COST, None)
        .expect("hardcoded Argon2 params must be valid")
}

/// 启动时播种的全局 CSPRNG
static SEEDED_RNG: OnceLock<Mutex<StdRng>> = OnceLock::new();

/// 在独立线程里播种全局 CSPRNG，由启动流程调用。
///
/// 设备无硬件熵源，内核 CRNG 需累积中断熵才能就绪（`random: crng init done`，
/// 实测约在开机 100 秒），在此之前 `getrandom(2)` 一直阻塞。播种放在独立线程
/// 无限期等待，既不占用 tokio worker，也不落在请求路径上。
pub fn spawn_rng_warmup() {
    std::thread::spawn(|| {
        let mut seed = <StdRng as SeedableRng>::Seed::default();
        if OsRng.try_fill_bytes(seed.as_mut()).is_err() {
            tracing::warn!("CSPRNG seeding failed, falling back to /dev/urandom");
            return;
        }
        let _ = SEEDED_RNG.set(Mutex::new(StdRng::from_seed(seed)));
        tracing::info!("CSPRNG seeded");
    });
}

/// 填充随机字节，任何情况下都不阻塞。
///
/// 每次调用都现查播种状态，不依赖任何时间假设。播种未完成时回退读
/// `/dev/urandom`：读取永不阻塞，此时输出由内核已用中断熵快速播种的 CRNG
/// （`crng_init=1`）产生，强度低于完全就绪状态，但这是 CRNG 就绪前系统能提供
/// 的最好熵源；播种一旦完成，后续调用自动改用 `SEEDED_RNG`。
fn fill_random(buf: &mut [u8]) {
    if let Some(rng) = SEEDED_RNG.get() {
        if let Ok(mut rng) = rng.lock() {
            rng.fill_bytes(buf);
            return;
        }
    }

    use std::io::Read;
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        if file.read_exact(buf).is_ok() {
            return;
        }
    }

    OsRng.fill_bytes(buf);
}

/// Argon2 哈希/校验是 CPU+内存密集型阻塞操作；放到 `spawn_blocking` 里跑，
/// 避免独占 tokio 工作线程导致其他并发请求被卡住。
pub async fn hash_password(password: &str) -> Result<String, String> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut salt_bytes = [0u8; 16];
        fill_random(&mut salt_bytes);
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| format!("Failed to generate salt: {}", e))?;
        let argon2 = Argon2::new(Algorithm::default(), Version::default(), new_password_params());
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| format!("Failed to hash password: {}", e))
    })
    .await
    .map_err(|e| format!("Password hashing task panicked: {}", e))?
}

pub async fn verify_password(password: &str, hash: &str) -> bool {
    let password = password.to_owned();
    let hash = hash.to_owned();
    tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    fill_random(&mut bytes);
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
