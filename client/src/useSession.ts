/**
 * 両ページ共通の接続フック。サーバーの snapshot をそのまま state に持ち、
 * 自分では盤面を書き換えない。表示は必ずサーバーの正規状態と一致する。
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  type Connection,
  type ConnectionStatus,
  connect,
  webSocketUrl,
} from "./connection";
import {
  APP_VERSION,
  type Command,
  type ServerMessage,
  type Snapshot,
} from "./protocol";

/**
 * 一度読み直したことを覚えておく鍵。読み直しても版が揃わない (ページが古いまま
 * 配られている等) ときに、読み込み直しを繰り返さないための歯止め。
 */
const RELOADED_FOR = "stream-shougi-board.reloaded-for";

/**
 * サーバーの版がページの版と違うのは、アプリを更新する前から開きっぱなしの
 * ページが再接続してきたとき。古い JS のまま描くと盤が壊れた形で配信に出るので、
 * 黙って読み直して新しいページに入れ替える。
 */
function reloadForNewApp(serverVersion: string): void {
  try {
    if (window.sessionStorage.getItem(RELOADED_FOR) === serverVersion) {
      console.warn(
        `StreamShougiBoard: ページは ${APP_VERSION}、アプリは ${serverVersion}。読み直しても揃わないので、ページを開き直してください。`,
      );
      return;
    }
    window.sessionStorage.setItem(RELOADED_FOR, serverVersion);
  } catch {
    // sessionStorage を使えない設定でも、読み直し自体はしてよい。
  }
  window.location.reload();
}

export interface SessionView {
  snapshot: Snapshot | null;
  status: ConnectionStatus;
  /** 直前に拒否されたコマンドの理由。操作ページが短く出す。 */
  rejected: string | null;
  send(command: Command): void;
}

export function useSession(): SessionView {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [rejected, setRejected] = useState<string | null>(null);
  const connectionRef = useRef<Connection | null>(null);

  useEffect(() => {
    const handleMessage = (message: ServerMessage) => {
      switch (message.type) {
        case "snapshot":
          if (message.appVersion !== APP_VERSION) {
            reloadForNewApp(message.appVersion);
            return;
          }
          setSnapshot(message);
          setRejected(null);
          return;
        case "rejected":
          setRejected(message.reason);
          return;
        case "pong":
          return;
      }
    };

    const connection = connect(webSocketUrl(), {
      onMessage: handleMessage,
      onStatus: setStatus,
    });
    connectionRef.current = connection;

    return () => {
      connectionRef.current = null;
      connection.close();
    };
  }, []);

  const send = useCallback((command: Command) => {
    connectionRef.current?.send(command);
  }, []);

  return { snapshot, status, rejected, send };
}
