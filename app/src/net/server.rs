//! loopback の HTTP / WebSocket サーバー。docs/security.md の防御をここで実装する。
//!
//! 127.0.0.1 にしか bind せず、Host と Origin を検証する。TLS も token も持たないのは、
//! 通信が同じ PC の中で閉じているため。LAN からの到達は bind 先で、
//! ブラウザ経由の悪意あるページからの接続は Origin 検証で塞ぐ。

use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST, ORIGIN, REFERRER_POLICY,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use rust_embed::RustEmbed;
use tokio::sync::broadcast::error::RecvError;

use crate::net::hub::{encode, Hub};
use crate::protocol::{parse_client_message, ClientMessage, ServerMessage, MAX_CLIENT_FRAME_BYTES};

pub const LOOPBACK_HOST: &str = "127.0.0.1";

/// client のビルド成果物。exe へ埋め込むので、実行時に外部ファイルを探しに行かない。
#[derive(RustEmbed)]
#[folder = "../client/static"]
#[include = "*.html"]
#[include = "*.css"]
#[include = "*.js"]
struct Assets;

/// 第三者ライセンスページのアセット名。CSP をこのページだけ別に決めるために使う。
const LICENSES_ASSET: &str = "licenses.html";

pub struct Started {
    pub port: u16,
}

struct AppState {
    hub: Arc<Hub>,
    port: u16,
}

/// 同じ PC から見た自分自身の名前だけを認める。
fn allowed_hosts(port: u16) -> [String; 2] {
    [
        format!("{LOOPBACK_HOST}:{port}"),
        format!("localhost:{port}"),
    ]
}

pub fn is_allowed_host(header: Option<&str>, port: u16) -> bool {
    header.is_some_and(|value| allowed_hosts(port).iter().any(|host| host == value))
}

/// Origin の無い WebSocket upgrade も拒む。
/// ブラウザは必ず付けてくるので、無いものは非ブラウザからの接続とみなす。
pub fn is_allowed_origin(header: Option<&str>, port: u16) -> bool {
    header.is_some_and(|value| {
        allowed_hosts(port)
            .iter()
            .any(|host| value == format!("http://{host}"))
    })
}

fn content_security_policy(port: u16) -> String {
    let sockets = allowed_hosts(port)
        .iter()
        .map(|host| format!("ws://{host}"))
        .collect::<Vec<_>>()
        .join(" ");
    [
        "default-src 'none'".to_owned(),
        "script-src 'self'".to_owned(),
        "style-src 'self'".to_owned(),
        "img-src 'self' data:".to_owned(),
        "font-src 'self'".to_owned(),
        format!("connect-src {sockets}"),
        "base-uri 'none'".to_owned(),
        "form-action 'none'".to_owned(),
        "frame-ancestors 'none'".to_owned(),
    ]
    .join("; ")
}

/// 第三者ライセンスページ用の CSP。
/// 生成物なので style をインラインで持つ代わりに、script も接続も一切許さない。
/// アプリ本体のページより厳しい。
const LICENSES_CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:; \
     base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// URL パス → アセット名。ページだけは拡張子なしで引けるようにする。
/// 任意のパスを触らせないため、埋め込み済みアセット名との完全一致だけを配る。
fn asset_name(path: &str) -> &str {
    match path {
        "/" | "/control" => "control.html",
        "/board" => "board.html",
        "/licenses" => LICENSES_ASSET,
        other => other.trim_start_matches('/'),
    }
}

fn content_type_of(name: &str) -> &'static str {
    if name.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if name.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if name.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn asset_response(name: &str, port: u16, method: &Method) -> Response {
    let Some(asset) = Assets::get(name) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let csp = if name == LICENSES_ASSET {
        LICENSES_CSP.to_owned()
    } else {
        content_security_policy(port)
    };

    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(asset.data.into_owned())
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type_of(name))
        // アプリを更新したのに OBS が古いページを握り続ける事故を防ぐ。
        .header(CACHE_CONTROL, "no-store")
        .header(CONTENT_SECURITY_POLICY, csp)
        .header(REFERRER_POLICY, "no-referrer")
        .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn header_str<'a>(headers: &'a HeaderMap, name: &axum::http::HeaderName) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn handle(State(state): State<Arc<AppState>>, request: Request<Body>) -> Response {
    let headers = request.headers().clone();
    if !is_allowed_host(header_str(&headers, &HOST), state.port) {
        return (StatusCode::FORBIDDEN, "forbidden host").into_response();
    }

    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    if path == "/ws" {
        if !is_allowed_origin(header_str(&headers, &ORIGIN), state.port) {
            return (StatusCode::FORBIDDEN, "forbidden origin").into_response();
        }
        return match WebSocketUpgrade::from_request(request).await {
            Ok(upgrade) => upgrade
                // 大きすぎるフレームは読む前に切る。受け付けるのは ping と短いコマンドだけ。
                .max_message_size(MAX_CLIENT_FRAME_BYTES)
                .max_frame_size(MAX_CLIENT_FRAME_BYTES)
                .on_upgrade(move |socket| subscribe(socket, state.hub.clone())),
            Err(rejection) => rejection.into_response(),
        };
    }

    if method != Method::GET && method != Method::HEAD {
        return (StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
    }

    asset_response(asset_name(&path), state.port, &method)
}

/// WebSocketUpgrade を Request から取り出すための小さな補助。
trait FromRequestExt: Sized {
    async fn from_request(request: Request<Body>) -> Result<Self, Response>;
}

impl FromRequestExt for WebSocketUpgrade {
    async fn from_request(request: Request<Body>) -> Result<Self, Response> {
        use axum::extract::FromRequestParts;
        let (mut parts, _) = request.into_parts();
        WebSocketUpgrade::from_request_parts(&mut parts, &())
            .await
            .map_err(IntoResponse::into_response)
    }
}

/// 接続 1 本の面倒を見る。接続直後は必ず現在状態を送り、以降は更新を中継する。
async fn subscribe(mut socket: WebSocket, hub: Arc<Hub>) {
    let mut updates = hub.subscribe();

    if socket
        .send(Message::text(hub.snapshot_json()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            update = updates.recv() => match update {
                Ok(payload) => {
                    if socket.send(Message::text(payload)).await.is_err() {
                        return;
                    }
                }
                // 溜め込んで取りこぼした接続は、現在状態を 1 件送り直せば追いつく。
                Err(RecvError::Lagged(_)) => {
                    if socket.send(Message::text(hub.snapshot_json())).await.is_err() {
                        return;
                    }
                }
                Err(RecvError::Closed) => return,
            },

            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let Some(reply) = handle_client_message(&hub, &text) {
                        if socket.send(Message::text(reply)).await.is_err() {
                            return;
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => return,
                // ping/pong と binary は受け付けない。無視して読み続ける。
                Some(Ok(_)) => {}
                Some(Err(_)) => return,
            },
        }
    }
}

/// 送り主にだけ返すべき応答があれば、その JSON を返す。
fn handle_client_message(hub: &Hub, text: &str) -> Option<String> {
    // 未知の type や壊れた JSON は黙って捨てる。
    let message = parse_client_message(text)?;

    if let ClientMessage::Ping { t } = message {
        return Some(encode(&ServerMessage::Pong { t }));
    }

    let rejected = hub.apply(message)?;
    Some(encode(&ServerMessage::Rejected { reason: rejected }))
}

pub fn router(hub: Arc<Hub>, port: u16) -> Router {
    Router::new()
        .route("/", any(handle))
        .fallback(any(handle))
        .with_state(Arc::new(AppState { hub, port }))
}

/// 既に bind 済みの listener を受け取る。実際のポートは呼び出し側が先に知る必要があるため。
pub async fn serve(
    listener: std::net::TcpListener,
    hub: Arc<Hub>,
    port: u16,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    axum::serve(listener, router(hub, port))
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = shutdown => {}
                _ = tokio::signal::ctrl_c() => {}
            }
        })
        .await?;
    Ok(())
}

/// 127.0.0.1 だけに bind し、実際に割り当たったポートと一緒に返す。
pub fn bind(port: u16) -> anyhow::Result<(std::net::TcpListener, Started)> {
    let listener = std::net::TcpListener::bind((LOOPBACK_HOST, port))?;
    let port = listener.local_addr()?.port();
    Ok((listener, Started { port }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 自分自身の名前だけを_host_として認める() {
        assert!(is_allowed_host(Some("127.0.0.1:16874"), 16874));
        assert!(is_allowed_host(Some("localhost:16874"), 16874));
        assert!(!is_allowed_host(Some("192.168.1.20:16874"), 16874));
        assert!(!is_allowed_host(Some("127.0.0.1:16875"), 16874));
        assert!(!is_allowed_host(Some("evil.example.com"), 16874));
        // Host の無いリクエストは拒否する。
        assert!(!is_allowed_host(None, 16874));
    }

    #[test]
    fn 同一_origin_だけを認める() {
        assert!(is_allowed_origin(Some("http://127.0.0.1:16874"), 16874));
        assert!(is_allowed_origin(Some("http://localhost:16874"), 16874));
        assert!(!is_allowed_origin(Some("https://127.0.0.1:16874"), 16874));
        assert!(!is_allowed_origin(Some("http://evil.example.com"), 16874));
        // Origin の無い upgrade も拒否する。
        assert!(!is_allowed_origin(None, 16874));
    }

    #[test]
    fn ページは拡張子なしで引ける() {
        assert_eq!(asset_name("/"), "control.html");
        assert_eq!(asset_name("/control"), "control.html");
        assert_eq!(asset_name("/board"), "board.html");
        assert_eq!(asset_name("/licenses"), "licenses.html");
        assert_eq!(asset_name("/board.js"), "board.js");
        // 埋め込み名との完全一致だけを配るので、ここで通っても Assets::get で落ちる。
        assert_eq!(asset_name("/../app/src/main.rs"), "../app/src/main.rs");
    }

    #[test]
    fn csp_は外部読み込みを禁じ_loopback_の_ws_だけを許す() {
        let csp = content_security_policy(16874);
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("connect-src ws://127.0.0.1:16874 ws://localhost:16874"));
    }

    #[test]
    fn ライセンスページの_csp_は本体より厳しい() {
        assert!(LICENSES_CSP.contains("style-src 'unsafe-inline'"));
        assert!(!LICENSES_CSP.contains("script-src"));
        assert!(!LICENSES_CSP.contains("connect-src"));
    }
}
