import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import markup from "./markup.html?raw";
import "./styles.css";

let opened = false;

type Bounds = { x: number; y: number; width: number; height: number };

function hostRect(): DOMRect | null {
  const host = document.getElementById("gphotos-host");
  if (!host) return null;
  const rect = host.getBoundingClientRect();
  if (rect.width < 2 || rect.height < 2) return null;
  return rect;
}

// Full-size content view: the webview frame includes the title bar, the CSS
// viewport does not. Child bounds use the frame; getBoundingClientRect uses the viewport.
async function frameInset(): Promise<{ x: number; y: number }> {
  const win = getCurrentWindow();
  const scale = await win.scaleFactor();
  const inner = (await win.innerSize()).toLogical(scale);
  return {
    x: Math.max(0, Math.round(inner.width - window.innerWidth)),
    y: Math.max(0, Math.round(inner.height - window.innerHeight)),
  };
}

async function hostBounds(): Promise<Bounds | null> {
  const rect = hostRect();
  if (!rect) return null;
  const inset = await frameInset();
  return {
    x: rect.left + inset.x,
    y: rect.top + inset.y,
    width: rect.width,
    height: rect.height,
  };
}

async function pushBounds(): Promise<void> {
  const bounds = await hostBounds();
  if (!bounds) return;
  await invoke("gphotos_set_bounds", bounds);
}

export async function syncGphotosVisibility(wantsVisible: boolean): Promise<void> {
  const dialogOpen = document.querySelector("dialog[open]") !== null;
  const tutorialOpen = !document.getElementById("gphotos-tutorial")?.classList.contains("hidden");
  const visible = wantsVisible && !dialogOpen && !tutorialOpen;
  if (!opened && !visible) return;
  await invoke("gphotos_set_visible", { visible });
  if (visible) await pushBounds();
}

export async function mountGphotos(host: HTMLElement): Promise<void> {
  const panel = host.querySelector("#gphotos-panel");
  if (!panel) return;
  panel.innerHTML = markup;
  const { startPanel } = await import("./panel.ts");
  await startPanel();

  const frame = host.querySelector("#gphotos-host");
  if (frame) {
    const observer = new ResizeObserver(() => {
      void pushBounds();
    });
    observer.observe(frame);
  }
  window.addEventListener("resize", () => {
    void pushBounds();
  });

  await pushBounds();
  const bounds = await hostBounds();
  await invoke("gphotos_open", bounds ?? { x: 0, y: 0, width: 1, height: 1 });
  opened = true;
  await syncGphotosVisibility(true);
}
