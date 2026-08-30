/**
 * ローカルサーバーへの WebSocket 接続。docs/protocol.md と対で読む。
 *
 * 切断は異常ではなく日常 (アプリ再起動、OBS のソース再読み込み) なので、
 * 黙って再接続し、snapshot を受け取った時点をもって同期成立とみなす。
 */

import type { ClientMessage, ServerMessage } from "./protocol";

const PING_INTERVAL_MS = 15_000;
const IDLE_TIMEOUT_MS = 30_000;
const BACKOFF_MIN_MS = 500;
const BACKOFF_MAX_MS = 15_000;
const SOCKET_OPEN = 1;
const SOCKET_CLOSING = 2;

/** WebSocket open ではなく、snapshot を受理して表示が正しくなった時点を connected と呼ぶ。 */
export type ConnectionStatus = "connected" | "disconnected";

export interface Connection {
  /** 接続していないときは黙って捨てる。遅れて届く操作より、届かない方が安全。 */
  send(message: ClientMessage): void;
  close(): void;
}

export interface ConnectionHandlers {
  onMessage(message: ServerMessage): void;
  onStatus?(status: ConnectionStatus): void;
}

export function webSocketUrl(): string {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/ws`;
}

export function connect(url: string, handlers: ConnectionHandlers): Connection {
  let socket: WebSocket | null = null;
  let closed = false;
  let backoff = BACKOFF_MIN_MS;
  let pingTimer: number | null = null;
  let reconnectTimer: number | null = null;
  let lastReceived = 0;
  let lastStatus: ConnectionStatus | null = null;

  function notifyStatus(status: ConnectionStatus): void {
    if (lastStatus === status) return;
    lastStatus = status;
    handlers.onStatus?.(status);
  }

  function clearPing(): void {
    if (pingTimer !== null) window.clearInterval(pingTimer);
    pingTimer = null;
  }

  function detach(target: WebSocket): void {
    target.onopen = null;
    target.onmessage = null;
    target.onclose = null;
    target.onerror = null;
    if (target.readyState < SOCKET_CLOSING) target.close();
    if (socket === target) socket = null;
  }

  function scheduleReconnect(target: WebSocket): void {
    if (closed || target !== socket) return;
    clearPing();
    detach(target);
    notifyStatus("disconnected");
    if (reconnectTimer !== null) window.clearTimeout(reconnectTimer);
    const jitter = 1 + (Math.random() * 0.4 - 0.2);
    reconnectTimer = window.setTimeout(open, backoff * jitter);
    backoff = Math.min(backoff * 2, BACKOFF_MAX_MS);
  }

  function open(): void {
    if (closed) return;
    reconnectTimer = null;
    const target = new WebSocket(url);
    socket = target;
    lastReceived = Date.now();

    target.onopen = () => {
      if (target !== socket) return;
      clearPing();
      pingTimer = window.setInterval(() => {
        if (Date.now() - lastReceived > IDLE_TIMEOUT_MS) {
          // pong が返らない。TCP は生きていても相手が死んでいる場合を拾う。
          scheduleReconnect(target);
          return;
        }
        if (target.readyState === SOCKET_OPEN) {
          target.send(JSON.stringify({ type: "ping", t: Date.now() }));
        }
      }, PING_INTERVAL_MS);
    };

    target.onmessage = (event) => {
      if (target !== socket) return;
      lastReceived = Date.now();
      try {
        const message = JSON.parse(String(event.data)) as ServerMessage;
        handlers.onMessage(message);
        // open しただけでは、直後に protocol 不一致で切られる場合を成功と誤認する。
        // snapshot を受け取れた時点で初めて backoff を戻す。
        if (message.type === "snapshot") {
          backoff = BACKOFF_MIN_MS;
          notifyStatus("connected");
        }
      } catch (error) {
        console.warn("StreamShougiBoard: failed to handle message", error);
        scheduleReconnect(target);
      }
    };

    target.onclose = () => scheduleReconnect(target);
    target.onerror = () => {
      // onclose が続けて呼ばれるので、ここでは何もしない。
    };
  }

  open();

  return {
    send(message) {
      if (socket === null || socket.readyState !== SOCKET_OPEN) return;
      socket.send(JSON.stringify(message));
    },
    close() {
      closed = true;
      if (reconnectTimer !== null) window.clearTimeout(reconnectTimer);
      reconnectTimer = null;
      clearPing();
      if (socket !== null) detach(socket);
    },
  };
}
