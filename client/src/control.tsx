import ReactDOM from "react-dom/client";
import { ControlPage } from "~/Control/component";

const root = document.getElementById("root");
if (root !== null) {
  // StrictMode は effect を二重に走らせて WebSocket を二重接続するので使わない。
  ReactDOM.createRoot(root).render(<ControlPage />);
}
