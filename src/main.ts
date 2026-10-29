import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { initOverview } from "./tabs/overview.ts";

type TabId = "overview" | "adware" | "autobackup" | "pricecreator" | "gphotos";

const mounted = new Set<TabId>();

function isTab(id: string | undefined): id is TabId {
  return id === "overview" || id === "adware" || id === "autobackup" || id === "pricecreator" || id === "gphotos";
}

async function ensureTab(id: TabId) {
  if (id === "overview" || mounted.has(id)) return;
  if (id === "adware") {
    const mod = await import("./tabs/adware/main.ts");
    mod.mountAdware(document.getElementById("tab-adware")!);
  } else if (id === "autobackup") {
    const mod = await import("./tabs/autobackup/mount.tsx");
    mod.mountAutoBackup(document.getElementById("tab-autobackup")!);
  } else if (id === "pricecreator") {
    const mod = await import("./tabs/pricecreator/mount.tsx");
    mod.mountPriceCreator(document.getElementById("tab-pricecreator")!);
  } else if (id === "gphotos") {
    const mod = await import("./tabs/gphotos/main.ts");
    await mod.mountGphotos(document.getElementById("tab-gphotos")!);
  } else {
    const _never: never = id;
    return _never;
  }
  mounted.add(id);
}

function showTab(id: TabId) {
  document.querySelectorAll<HTMLElement>(".top-tab").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.tab === id);
  });
  document.querySelectorAll<HTMLElement>(".tab-panel").forEach((panel) => {
    panel.classList.toggle("hidden", panel.dataset.tabPanel !== id);
  });
  void (async () => {
    await ensureTab(id);
    const { syncGphotosVisibility } = await import("./tabs/gphotos/main.ts");
    await syncGphotosVisibility(id === "gphotos");
  })();
}

document.querySelectorAll<HTMLElement>(".top-tab").forEach((btn) => {
  btn.addEventListener("click", () => {
    const id = btn.dataset.tab;
    if (isTab(id)) showTab(id);
  });
});

void listen("app-close-requested", () => {
  void invoke("adware_allow_app_close");
});

document.getElementById("btn-app-settings")?.addEventListener("click", () => {
  void (async () => {
    await ensureTab("adware");
    const mod = await import("./tabs/adware/main.ts");
    await mod.openAppSettings();
  })();
});

document.getElementById("btn-retry-connection-global")?.addEventListener("click", () => {
  void (async () => {
    showTab("adware");
    await ensureTab("adware");
    const mod = await import("./tabs/adware/main.ts");
    await mod.retryConnection();
  })();
});

document.addEventListener(
  "toggle",
  (event) => {
    if (!(event.target instanceof HTMLDialogElement)) return;
    const onGphotos = document.querySelector(".top-tab.active")?.getAttribute("data-tab") === "gphotos";
    void import("./tabs/gphotos/main.ts").then((mod) => mod.syncGphotosVisibility(onGphotos));
  },
  true,
);

const gphotosLog = document.getElementById("gphotos-show-log") as HTMLInputElement | null;
gphotosLog?.addEventListener("change", () => {
  void invoke("gphotos_set_dev_mode", { enabled: gphotosLog.checked });
});
document.getElementById("btn-app-settings")?.addEventListener("click", () => {
  void invoke<{ enabled: boolean }>("gphotos_get_dev_mode")
    .then((state) => {
      if (gphotosLog) gphotosLog.checked = state.enabled;
    })
    .catch(() => undefined);
});

void initOverview();
