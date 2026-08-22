//! Web 终端（ttyd）反向代理
//!
//! ttyd 以 `-i lo -b /api/terminal/proxy -H X-Remote-User` 启动，自身不做鉴权，
//! 改由本模块在请求通过管理后台鉴权中间件后注入该请求头放行。请求路径原样透传，
//! 由 ttyd 依据 `-b` 自行处理。
//!
//! `-i lo` 不可省略：`-H` 只要求请求带上指定的头，头值由客户端自由设置，ttyd 一旦
//! 监听在非回环地址，局域网内任何人带上该头就能取得 root 终端。

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tracing::{debug, error, info, warn};

use crate::state::AppState;

/// ttyd 监听地址
const TTYD_ADDR: &str = "127.0.0.1:7681";
/// ttyd 监听端口，需与 `TTYD_ADDR` 一致
pub const TTYD_PORT: u16 = 7681;
/// ttyd 的 `-i` 监听接口，需与 start.sh 中的参数一致
pub const TTYD_BIND_INTERFACE: &str = "lo";
/// ttyd 的 `-b` 挂载路径，需与 start.sh 中的参数一致
pub const TTYD_BASE_PATH: &str = "/api/terminal/proxy";
/// ttyd 的 `-H` 信任头名称，需与 start.sh 中的参数一致
pub const TTYD_AUTH_HEADER: &str = "X-Remote-User";
/// 注入给 ttyd 的用户标识，仅用于满足 `-H` 校验
const TTYD_AUTH_USER: &str = "udx710";

/// 小写形式，用于 HeaderMap 读写
const AUTH_HEADER_KEY: &str = "x-remote-user";
const PROTOCOL_HEADER_KEY: &str = "sec-websocket-protocol";

/// 逐跳头及本地会话凭据，代理时不转发给 ttyd
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
    "authorization",
    "cookie",
];

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP.contains(&name)
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

fn bad_gateway(message: String) -> Response {
    warn!(error = %message, "ttyd proxy failed");
    (StatusCode::BAD_GATEWAY, message).into_response()
}

/// GET/POST /api/terminal/proxy/* - 转发 ttyd 的静态资源与 token 接口
pub async fn terminal_proxy_http(req: Request) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path_and_query = uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or(TTYD_BASE_PATH);
    let target = format!("http://{}{}", TTYD_ADDR, path_and_query);

    let mut headers = HeaderMap::new();
    for (name, value) in req.headers() {
        if !is_hop_by_hop(name.as_str()) {
            headers.insert(name.clone(), value.clone());
        }
    }
    headers.insert(AUTH_HEADER_KEY, HeaderValue::from_static(TTYD_AUTH_USER));

    let body = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => return bad_gateway(format!("读取请求体失败: {}", e)),
    };

    let upstream = match http_client()
        .request(method, &target)
        .headers(headers)
        .body(body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => return bad_gateway(format!("连接 ttyd 失败: {}", e)),
    };

    let status = upstream.status();
    let mut resp_headers = HeaderMap::new();
    for (name, value) in upstream.headers() {
        if !is_hop_by_hop(name.as_str()) {
            resp_headers.insert(name.clone(), value.clone());
        }
    }

    let bytes = match upstream.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => return bad_gateway(format!("读取 ttyd 响应失败: {}", e)),
    };

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    response
}

/// 并发终端隧道上限，每条隧道对应一个 ttyd 会话（一个 shell 进程）
const MAX_TUNNELS: usize = 4;
/// 隧道建立后回查会话有效性的间隔
const SESSION_CHECK_INTERVAL: Duration = Duration::from_secs(30);

fn tunnel_gate() -> &'static Arc<Semaphore> {
    static GATE: OnceLock<Arc<Semaphore>> = OnceLock::new();
    GATE.get_or_init(|| Arc::new(Semaphore::new(MAX_TUNNELS)))
}

/// 未启用鉴权时无会话可言，一律视为有效
fn session_still_valid(state: &AppState, token: Option<&str>) -> bool {
    if !state.config_manager.get_auth().enabled {
        return true;
    }
    token.is_some_and(|token| state.session_store.validate(token))
}

/// GET /api/terminal/proxy/ws - 将终端 WebSocket 隧道到 ttyd
pub async fn terminal_proxy_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
) -> Response {
    let Ok(permit) = tunnel_gate().clone().try_acquire_owned() else {
        warn!(max = MAX_TUNNELS, "Rejected terminal tunnel: too many concurrent sessions");
        return (StatusCode::SERVICE_UNAVAILABLE, "终端会话数已达上限").into_response();
    };

    // 升级只在这一刻经过鉴权中间件，之后由隧道自己按 token 跟随会话生命周期
    let token = crate::auth::extract_session_token(&headers);

    // ttyd 客户端使用 "tty" 子协议，握手两端都必须带上
    let protocol = headers
        .get(PROTOCOL_HEADER_KEY)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let upstream_protocol = protocol.clone();
    let upgrade = match protocol {
        Some(p) => ws.protocols([p]),
        None => ws,
    };

    upgrade.on_upgrade(move |socket| tunnel(socket, upstream_protocol, state, token, permit))
}

/// 在浏览器与 ttyd 之间双向搬运 WebSocket 消息，任一端断开或会话失效即整体结束
async fn tunnel(
    client: WebSocket,
    protocol: Option<String>,
    state: AppState,
    token: Option<String>,
    _permit: OwnedSemaphorePermit,
) {
    let url = format!("ws://{}{}/ws", TTYD_ADDR, TTYD_BASE_PATH);

    let mut request = match url.as_str().into_client_request() {
        Ok(request) => request,
        Err(e) => {
            warn!(error = %e, "Invalid ttyd websocket url");
            return;
        }
    };
    request
        .headers_mut()
        .insert(AUTH_HEADER_KEY, HeaderValue::from_static(TTYD_AUTH_USER));
    if let Some(p) = protocol.as_deref().and_then(|p| HeaderValue::from_str(p).ok()) {
        request.headers_mut().insert(PROTOCOL_HEADER_KEY, p);
    }

    let upstream = match tokio_tungstenite::connect_async(request).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            warn!(error = %e, "Failed to connect ttyd websocket");
            return;
        }
    };

    let (mut upstream_tx, mut upstream_rx) = upstream.split();
    let (mut client_tx, mut client_rx) = client.split();

    let client_to_ttyd = async {
        while let Some(Ok(msg)) = client_rx.next().await {
            if upstream_tx.send(to_tungstenite(msg)).await.is_err() {
                break;
            }
        }
        let _ = upstream_tx.close().await;
    };

    let ttyd_to_client = async {
        while let Some(Ok(msg)) = upstream_rx.next().await {
            let Some(msg) = to_axum(msg) else { continue };
            if client_tx.send(msg).await.is_err() {
                break;
            }
        }
        let _ = client_tx.close().await;
    };

    // 隧道建立后不再经过鉴权中间件，退出登录、改密码、会话过期需要靠这里主动断开
    let session_watch = async {
        let mut ticker = tokio::time::interval(SESSION_CHECK_INTERVAL);
        ticker.tick().await; // interval 的首次 tick 立即完成
        loop {
            ticker.tick().await;
            if !session_still_valid(&state, token.as_deref()) {
                info!("Terminal session revoked, closing tunnel");
                break;
            }
        }
    };

    // 任一分支结束时其余 future 一并被丢弃，两侧套接字随之关闭
    tokio::select! {
        _ = client_to_ttyd => {}
        _ = ttyd_to_client => {}
        _ = session_watch => {}
    }

    debug!("ttyd websocket tunnel closed");
}

/// 关闭帧只保留语义，丢弃 code 与 reason，终端场景无需透传
fn to_tungstenite(msg: AxumMessage) -> TungMessage {
    match msg {
        AxumMessage::Text(text) => TungMessage::Text(text.as_str().into()),
        AxumMessage::Binary(data) => TungMessage::Binary(data),
        AxumMessage::Ping(data) => TungMessage::Ping(data),
        AxumMessage::Pong(data) => TungMessage::Pong(data),
        AxumMessage::Close(_) => TungMessage::Close(None),
    }
}

/// Frame 变体只在裸帧模式下出现，正常客户端读取时不会产生，直接忽略
fn to_axum(msg: TungMessage) -> Option<AxumMessage> {
    match msg {
        TungMessage::Text(text) => Some(AxumMessage::Text(text.as_str().into())),
        TungMessage::Binary(data) => Some(AxumMessage::Binary(data)),
        TungMessage::Ping(data) => Some(AxumMessage::Ping(data)),
        TungMessage::Pong(data) => Some(AxumMessage::Pong(data)),
        TungMessage::Close(_) => Some(AxumMessage::Close(None)),
        TungMessage::Frame(_) => None,
    }
}

// ============ ttyd 进程管理 ============

/// 探测 ttyd 是否已按代理模式运行
///
/// 双向确认：不带认证头必须被拒（ttyd 1.7.7 返回 407），带头才返回 200。只探带头的
/// 那一次无法区分「代理模式」与「完全没有鉴权」，后者同样返回 200。
async fn probe_ttyd_proxy_mode() -> bool {
    let url = format!("http://{}{}/", TTYD_ADDR, TTYD_BASE_PATH);

    let anonymous = http_client()
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    match anonymous {
        // 无凭据即放行，说明 ttyd 没有开启 -H 校验，必须当作未就绪处理
        Ok(resp) if resp.status() == StatusCode::OK => {
            warn!("ttyd accepts requests without the auth header");
            return false;
        }
        Ok(_) => {}
        // 端口还没起来
        Err(_) => return false,
    }

    http_client()
        .get(&url)
        .header(TTYD_AUTH_HEADER, TTYD_AUTH_USER)
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .is_ok_and(|resp| resp.status() == StatusCode::OK)
}

/// 检查 ttyd 是否监听在回环以外的地址
///
/// `-H` 只校验请求头是否存在，头值由客户端自由设置。ttyd 一旦绑在非回环地址，
/// 局域网内任何人带上该头即可取得 root 终端，完全绕过管理后台鉴权。
fn is_ttyd_publicly_bound() -> bool {
    // /proc/net/tcp 每行字段依次为 sl、local_address、rem_address、st…，
    // local_address 是「小端十六进制 IP:大端十六进制端口」，st 为 0A 表示 LISTEN
    fn has_public_listener(content: &str, loopback: &str) -> bool {
        content
            .lines()
            .skip(1)
            .filter_map(|line| {
                let mut fields = line.split_whitespace().skip(1);
                let local = fields.next()?;
                let state = fields.nth(1)?;
                Some((local, state))
            })
            .any(|(local, state)| {
                let Some((addr, port)) = local.split_once(':') else {
                    return false;
                };
                state == "0A"
                    && u16::from_str_radix(port, 16) == Ok(TTYD_PORT)
                    && !addr.eq_ignore_ascii_case(loopback)
            })
    }

    // IPv4 回环为 127.0.0.1，IPv6 回环为 ::1；IPv6 的 `::`（全零）属于对外监听
    let sources = [
        ("/proc/net/tcp", "0100007F"),
        ("/proc/net/tcp6", "00000000000000000000000001000000"),
    ];

    sources.into_iter().any(|(path, loopback)| {
        std::fs::read_to_string(path).is_ok_and(|content| has_public_listener(&content, loopback))
    })
}

/// 重启 ttyd 并确认它以代理模式起来
///
/// 每次重启后每 0.5 秒探测一次，超时 15 秒，最多重试 3 次。
pub async fn restart_ttyd_verified() -> Result<(), String> {
    use tokio::time::sleep;

    const CHECK_INTERVAL_MS: u64 = 500;
    const MAX_WAIT_MS: u64 = 15_000;
    const MAX_RESTARTS: usize = 3;

    for attempt in 1..=MAX_RESTARTS {
        // Command::status() 是同步阻塞调用，必须丢进 spawn_blocking，否则会
        // 独占当前 tokio 工作线程，在这台核心数很少的设备上足以拖住其他并发
        // 请求（包括登录接口）
        let _ = tokio::task::spawn_blocking(|| {
            std::process::Command::new("sh")
                .arg("-c")
                .arg("pkill ttyd; true")
                .status()
        })
        .await;

        sleep(Duration::from_millis(200)).await;

        let start_result = tokio::task::spawn_blocking(|| {
            std::process::Command::new("sh")
                .arg(crate::config::TTYD_START_SCRIPT_PATH)
                .status()
        })
        .await;

        match start_result {
            Ok(Err(e)) => warn!(attempt, error = %e, "Failed to run ttyd start script"),
            Err(e) => warn!(attempt, error = %e, "ttyd start script task panicked"),
            Ok(Ok(_)) => {}
        }

        let mut elapsed_ms = 0u64;
        while elapsed_ms < MAX_WAIT_MS {
            sleep(Duration::from_millis(CHECK_INTERVAL_MS)).await;
            elapsed_ms += CHECK_INTERVAL_MS;
            if probe_ttyd_proxy_mode().await {
                info!(attempt, elapsed_ms, "ttyd restarted in proxy mode");
                return Ok(());
            }
        }

        warn!(attempt, "ttyd did not come up within 15s, will retry");
    }

    Err(format!(
        "ttyd 重启失败：{MAX_RESTARTS} 次尝试均未能在 15 秒内以代理模式启动，请检查 start.sh 及 ttyd 版本"
    ))
}

/// 启动时校准 ttyd：修正 start.sh 参数，实例不满足「代理模式 + 绑定回环」则重启
///
/// loader.sh 几乎同时拉起 ttyd 和本进程，首次探测可能早于 ttyd 监听端口，
/// 因此先在 5 秒窗口内反复探测，确认确实不是代理模式才重启，避免无谓重启。
///
/// 无论探测结果如何都会先落一条 iptables 兜底规则：start.sh 格式无法识别时参数
/// 注入会跳过，此时 `-i lo` 不会生效，只剩这条规则挡住局域网直连。
pub async fn ensure_ttyd_proxy_runtime() {
    use tokio::time::sleep;

    const SETTLE_INTERVAL_MS: u64 = 500;
    const SETTLE_WINDOW_MS: u64 = 5_000;

    crate::iptables::ensure_ttyd_port_protected().await;

    let script_managed = match crate::config::ensure_ttyd_proxy_mode() {
        Ok(managed) => managed,
        Err(e) => {
            warn!(error = %e, "Failed to patch ttyd start script");
            false
        }
    };

    let mut proxy_mode = false;
    let mut waited_ms = 0u64;
    while waited_ms < SETTLE_WINDOW_MS {
        if probe_ttyd_proxy_mode().await {
            proxy_mode = true;
            break;
        }
        sleep(Duration::from_millis(SETTLE_INTERVAL_MS)).await;
        waited_ms += SETTLE_INTERVAL_MS;
    }

    // 运行中的实例可能仍绑在 0.0.0.0：`-i lo` 本次才写进 start.sh，对已启动的进程
    // 不生效。此时即使代理模式探测通过也必须重启，否则要等到下次开机才收口
    let publicly_bound = is_ttyd_publicly_bound();
    if publicly_bound {
        error!(
            port = TTYD_PORT,
            "ttyd is listening on a non-loopback address; anyone on the LAN can reach a root shell"
        );
    }

    if proxy_mode && !publicly_bound {
        debug!(waited_ms, "ttyd already running in proxy mode on loopback");
        return;
    }

    if publicly_bound && !script_managed {
        warn!("ttyd start script is not managed, restarting would not rebind it to loopback");
        return;
    }

    info!(proxy_mode, publicly_bound, "Restarting ttyd");
    if let Err(e) = restart_ttyd_verified().await {
        warn!(error = %e, "Failed to bring ttyd into proxy mode");
        return;
    }

    if is_ttyd_publicly_bound() {
        error!(
            port = TTYD_PORT,
            "ttyd is still bound to a non-loopback address after restart"
        );
    }
}
