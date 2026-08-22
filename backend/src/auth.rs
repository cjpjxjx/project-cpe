//! 密码鉴权模块
//!
//! 提供密码哈希、session token 生成与校验、以及一个全局 axum 中间件。
//! Session 只存内存，不落库，设备重启即失效。

use std::collections::HashMap;
use std::net::IpAddr;
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
/// 剩余有效期低于此值时，校验会顺带把会话续到完整 TTL。
/// 取 TTL 的一半，避免每个请求都去抢写锁。
const SESSION_RENEW_AFTER: Duration = Duration::from_secs(12 * 60 * 60);

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

/// 会话校验结果
pub struct SessionCheck {
    pub valid: bool,
    /// 本次校验是否续了期。续期后需要随响应重新下发 Set-Cookie，
    /// 否则浏览器侧仍按登录时的 Max-Age 到点丢弃 cookie
    pub renewed: bool,
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

    /// 校验并在剩余有效期不足时续期，实现滑动过期。
    ///
    /// 固定 24 小时绝对过期会在用户正操作时突然失效（页面轮询会立刻撞上 401，
    /// 表单填到一半被踢到登录页），持续使用的会话应当一直有效。
    pub fn validate_renewing(&self, token: &str) -> SessionCheck {
        let now = Instant::now();

        // 快路径：绝大多数请求离续期阈值还很远，只用读锁
        {
            let sessions = self.sessions.read().unwrap();
            match sessions.get(token) {
                None => return SessionCheck { valid: false, renewed: false },
                Some(expires_at) => {
                    if *expires_at <= now {
                        return SessionCheck { valid: false, renewed: false };
                    }
                    if *expires_at - now > SESSION_RENEW_AFTER {
                        return SessionCheck { valid: true, renewed: false };
                    }
                }
            }
        }

        // 读锁释放到写锁获取之间，会话可能已被登出或改密码清掉
        let mut sessions = self.sessions.write().unwrap();
        match sessions.get_mut(token) {
            Some(expires_at) if *expires_at > now => {
                *expires_at = now + SESSION_TTL;
                SessionCheck { valid: true, renewed: true }
            }
            _ => SessionCheck { valid: false, renewed: false },
        }
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

    // 这两个端点会主动废弃会话：登出删除当前会话并下发清除 cookie，改密码/开关
    // 鉴权会清空全部会话。它们一律不续期——续了期却不能下发 cookie 的话，
    // 服务端已经把剩余时间推回满值，此后 12 小时都不会再触发续期，
    // 浏览器侧 cookie 反而等不到刷新，到点照样被丢弃
    let skip_renewal = path == "/api/auth/logout" || path == "/api/auth/config";

    let token = extract_session_token(req.headers());
    let check = match token.as_deref() {
        None => SessionCheck { valid: false, renewed: false },
        Some(token) if skip_renewal => SessionCheck {
            valid: state.session_store.validate(token),
            renewed: false,
        },
        Some(token) => state.session_store.validate_renewing(token),
    };

    if !check.valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("未登录或会话已过期")),
        )
            .into_response();
    }

    let mut response = next.run(req).await;

    // 服务端续期后必须同步刷新 cookie 的 Max-Age，否则浏览器仍按登录时的
    // 到期时间丢弃 cookie，滑动过期就形同虚设
    if check.renewed {
        if let Some(token) = token.as_deref() {
            let cookie = build_set_cookie_header(token);
            if !cookie.is_empty() {
                response
                    .headers_mut()
                    .append(axum::http::header::SET_COOKIE, cookie);
            }
        }
    }

    response
}

// ============ 登录接口保护 ============

/// 登录校验并发上限：Argon2 按哈希串内嵌参数分配 8 ~ 19 MiB 内存，不限制的话
/// 高并发登录请求可以打爆设备内存
const LOGIN_MAX_CONCURRENCY: usize = 2;
/// 单个来源在 `LOGIN_WINDOW` 内连续失败达到该次数即锁定
const LOGIN_FAILURE_THRESHOLD: u32 = 10;
/// 失败计数窗口，也是锁定时长
const LOGIN_WINDOW: Duration = Duration::from_secs(10 * 60);
/// 跟踪的来源 IP 上限，超出时淘汰最久未活动的记录
const LOGIN_TRACKED_SOURCES_MAX: usize = 256;

/// 登录响应的固定最小耗时，抹平各条校验路径之间的时序差异
pub const LOGIN_MIN_LATENCY: Duration = Duration::from_millis(300);
/// 口令长度上限
pub const PASSWORD_MAX_BYTES: usize = 128;
/// 启用鉴权时的口令最小长度
pub const PASSWORD_MIN_BYTES: usize = 8;

/// 登录并发闸门，取不到许可直接拒绝，不排队
pub fn login_gate() -> &'static tokio::sync::Semaphore {
    static LOGIN_GATE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    LOGIN_GATE.get_or_init(|| tokio::sync::Semaphore::new(LOGIN_MAX_CONCURRENCY))
}

struct FailureRecord {
    count: u32,
    locked_until: Option<Instant>,
    last_seen: Instant,
}

fn login_failures() -> &'static Mutex<HashMap<IpAddr, FailureRecord>> {
    static FAILURES: OnceLock<Mutex<HashMap<IpAddr, FailureRecord>>> = OnceLock::new();
    FAILURES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 该来源剩余的锁定时间，`None` 表示未锁定
pub fn login_lock_remaining(ip: IpAddr) -> Option<Duration> {
    let map = login_failures().lock().ok()?;
    map.get(&ip)?
        .locked_until
        .and_then(|until| until.checked_duration_since(Instant::now()))
}

/// 记录一次失败，返回累计失败次数与本次触发的锁定时长
pub fn record_login_failure(ip: IpAddr) -> (u32, Option<Duration>) {
    let now = Instant::now();
    let Ok(mut map) = login_failures().lock() else {
        return (0, None);
    };

    prune_login_failures(&mut map, now);

    let record = map.entry(ip).or_insert(FailureRecord {
        count: 0,
        locked_until: None,
        last_seen: now,
    });
    record.count += 1;
    record.last_seen = now;

    if record.count < LOGIN_FAILURE_THRESHOLD {
        return (record.count, None);
    }

    record.locked_until = Some(now + LOGIN_WINDOW);
    (record.count, Some(LOGIN_WINDOW))
}

pub fn clear_login_failures(ip: IpAddr) {
    if let Ok(mut map) = login_failures().lock() {
        map.remove(&ip);
    }
}

/// 丢弃已超过窗口期的记录，并给表设容量上限
fn prune_login_failures(map: &mut HashMap<IpAddr, FailureRecord>, now: Instant) {
    map.retain(|_, record| record.last_seen + LOGIN_WINDOW > now);
    while map.len() >= LOGIN_TRACKED_SOURCES_MAX {
        let Some(oldest) = map
            .iter()
            .min_by_key(|(_, record)| record.last_seen)
            .map(|(ip, _)| *ip)
        else {
            break;
        };
        map.remove(&oldest);
    }
}

/// 常量时间比较。长度不同时仍会提前返回（长度本身不是秘密），内容比较不短路，
/// 避免「用户名前缀正确到第几位」被时序区分出来。
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
