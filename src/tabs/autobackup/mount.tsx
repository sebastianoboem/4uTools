import { createRoot, type Root } from "react-dom/client";
import App from "./App";
import "./App.css";

let root: Root | null = null;

export function mountAutoBackup(host: HTMLElement) {
  if (!root) root = createRoot(host);
  root.render(<App />);
}
