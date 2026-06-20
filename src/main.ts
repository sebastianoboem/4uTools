import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

interface AdbDevice {
  serial: string;
  state: string | { unknown: string };
  model?: string;
}

interface StorageBreakdown {
  system: number;
  apps: number;
  photos: number;
  audio: number;
  videos: number;
  downloads: number;
  other: number;
  free: number;
  total: number;
}

interface DetailRow {
  label: string;
  value: string;
  note?: string;
}

interface DetailSection {
  title: string;
  rows: DetailRow[];
}

interface DeviceSummary {
  deviceName: string;
  brand: string;
  model: string;
  product: string;
  region: string;
  activationStatus: string;
  manufacturingDate: string;
  androidVersion: string;
  securityPatch: string;
  serial: string;
  imei: string;
  imei2: string;
  rootStatus: string;
  frpStatus: string;
  bootloaderStatus: string;
  storageTotal: string;
  storageBreakdown: StorageBreakdown;
  batteryLevel: number;
  batteryHealth: number;
  batteryDesignCapacityMah: number;
  batteryMaxCapacityMah: number;
  batteryTemperature: string;
  batteryChargingPower: string;
  batteryTechnology: string;
  chargeCycles: number;
  chargingStatus: string;
  isCharging: boolean;
  verificationStatus: string;
  verificationScore: number;
  deviceDetails: DetailSection[];
  batteryDetails: DetailSection[];
  storageDetails: DetailSection[];
  verificationChecks: VerificationCheck[];
}

interface VerificationCheck {
  item: string;
  factoryValue?: string;
  readValue: string;
  result: string;
  note?: string;
}

interface MirrorFrame {
  serial: string;
  width: number;
  height: number;
  imageDataUrl: string;
}

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const screenEmpty = $("screen-empty");
const screenDashboard = $("screen-dashboard");
const errorBanner = $("error-banner");
const hideSerialCheck = $<HTMLInputElement>("hide-serial");
const mirrorImg = $<HTMLImageElement>("mirror-img");
const phoneScreen = $("phone-screen");

let devices: AdbDevice[] = [];
let selectedSerial: string | null = null;
let loading = false;
let currentSummary: DeviceSummary | null = null;
let mirrorDeviceWidth = 0;
let mirrorDeviceHeight = 0;

function deviceState(d: AdbDevice): string {
  if (typeof d.state === "string") return d.state;
  return "unknown";
}

function isAuthorized(d: AdbDevice): boolean {
  return deviceState(d) === "device";
}

function mask(value: string): string {
  if (!hideSerialCheck.checked || !value || value === "N/A") return value;
  if (value.length <= 4) return "****";
  return (
    value.slice(0, 2) +
    "*".repeat(Math.min(value.length - 4, 8)) +
    value.slice(-2)
  );
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function showError(msg: string) {
  if (!msg) {
    errorBanner.classList.add("hidden");
    errorBanner.textContent = "";
    return;
  }
  errorBanner.textContent = msg;
  errorBanner.classList.remove("hidden");
}

function hideAllScreens() {
  screenEmpty.classList.add("hidden");
  screenDashboard.classList.add("hidden");
}

function updateScreens() {
  hideAllScreens();

  if (!selectedSerial) {
    screenEmpty.classList.remove("hidden");
    return;
  }

  if (currentSummary) {
    screenDashboard.classList.remove("hidden");
    void startMirrorPreview();
  } else {
    screenEmpty.classList.remove("hidden");
  }
}

function syncSelectedDevice(): boolean {
  const auth = devices.find(isAuthorized);
  const next = auth?.serial ?? null;
  if (next === selectedSerial) return false;
  selectedSerial = next;
  return true;
}

async function applyDeviceList() {
  const prev = selectedSerial;
  syncSelectedDevice();
  if (selectedSerial) {
    if (selectedSerial !== prev || !currentSummary) await refreshSummary();
    else updateScreens();
  } else {
    await stopMirrorPreview();
    currentSummary = null;
    updateScreens();
  }
}

function gaugeStrokeColor(health: number): string {
  if (health >= 80) return "#22c55e";
  if (health >= 50) return "#f97316";
  return "#ef4444";
}

function batteryLevelColor(level: number, charging: boolean): string {
  if (charging) return "#22c55e";
  if (level <= 20) return "#ef4444";
  if (level <= 50) return "#f97316";
  return "#3b82f6";
}

function modalFillAnimation(
  keyframes: string,
  delayMs: number,
  durationMs: number,
) {
  const delay = delayMs / 1000;
  const duration = durationMs / 1000;
  return `${keyframes} ${duration}s linear ${delay}s backwards`;
}

function batteryMaxCapacityPct(s: DeviceSummary): number {
  if (s.batteryDesignCapacityMah > 0 && s.batteryMaxCapacityMah > 0) {
    return Math.min(
      100,
      (s.batteryMaxCapacityMah / s.batteryDesignCapacityMah) * 100,
    );
  }
  return Math.min(100, Math.max(0, s.batteryHealth));
}

function renderBatteryModalVisual(s: DeviceSummary): string {
  const level = Math.min(100, Math.max(0, s.batteryLevel));
  const maxPct = batteryMaxCapacityPct(s);
  const animMs = BATTERY_ANIMATION_MS;
  const fillColor = batteryLevelColor(level, s.isCharging);
  const maxColor = gaugeStrokeColor(maxPct);
  const maxLabel =
    s.batteryMaxCapacityMah > 0
      ? `${s.batteryMaxCapacityMah} mAh`
      : `${Math.round(maxPct)}%`;

  return `
    <div class="battery-modal-visual">
      <div class="battery-capacity-mini">
        <span class="battery-capacity-label">Max capacity</span>
        <div class="battery-capacity-track">
          <div class="battery-capacity-fill"
            data-delay="0"
            data-duration="${animMs}"
            data-keyframes="battery-grow-x"
            style="width:${maxPct}%;background:${maxColor};animation:${modalFillAnimation("battery-grow-x", 0, animMs)}"></div>
        </div>
        <span class="battery-capacity-value muted">${maxLabel}</span>
      </div>
      <div class="battery-vertical-unit">
        <div class="battery-vertical-cap"></div>
        <div class="battery-vertical-shell">
          <div class="battery-vertical-fill${s.isCharging ? " charging" : ""}"
            data-delay="0"
            data-duration="${animMs}"
            data-keyframes="battery-grow-y"
            style="height:${level}%;background:${fillColor};animation:${modalFillAnimation("battery-grow-y", 0, animMs)}"></div>
        </div>
        <span class="battery-vertical-value">${level}%</span>
      </div>
    </div>`;
}

function replayBatteryModalAnimation(container: ParentNode) {
  container
    .querySelectorAll(".battery-capacity-fill, .battery-vertical-fill")
    .forEach((el) => {
      const node = el as HTMLElement;
      const delayMs = Number(node.dataset.delay ?? 0);
      const durationMs = Number(node.dataset.duration ?? 0);
      const keyframes = node.dataset.keyframes ?? "battery-grow-y";
      const anim = modalFillAnimation(keyframes, delayMs, durationMs);
      node.style.animation = "none";
      void node.offsetWidth;
      node.style.animation = anim;
    });
}

function renderGauge(value: number, size = 96, caption = "Health") {
  const r = (size - 14) / 2;
  const circ = 2 * Math.PI * r;
  const cx = size / 2;
  const offset = circ - (value / 100) * circ;
  const color = gaugeStrokeColor(value);

  return `
    <div class="gauge" style="width:${size}px;height:${size}px">
      <svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
        <circle class="gauge-track" cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke="#f3f4f6" stroke-width="9"/>
        <circle class="gauge-arc" cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke="${color}" stroke-width="9"
          stroke-linecap="round" stroke-dasharray="${circ}" stroke-dashoffset="${offset}"/>
      </svg>
      <div class="gauge-label">
        <span class="gauge-value">${value}%</span>
        <span class="gauge-caption">${caption}</span>
      </div>
    </div>`;
}

function renderStorageDonut(usedPct: number) {
  const size = 72;
  const stroke = 8;
  const r = (size - stroke) / 2;
  const circ = 2 * Math.PI * r;
  const cx = size / 2;
  const offset = circ - (Math.min(100, Math.max(0, usedPct)) / 100) * circ;
  const label = `${Math.round(usedPct)}%`;

  return `
    <svg viewBox="0 0 ${size} ${size}" style="transform:rotate(-90deg)">
      <circle class="storage-donut-track" cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke-width="${stroke}"/>
      <circle class="storage-donut-arc" cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke="#3b82f6" stroke-width="${stroke}"
        stroke-linecap="round" stroke-dasharray="${circ}" stroke-dashoffset="${circ}" data-offset="${offset}"/>
    </svg>
    <span class="storage-donut-overlay">${label}</span>`;
}

function animateDonutArc(container: HTMLElement) {
  const arc = container.querySelector(".storage-donut-arc") as SVGCircleElement | null;
  if (!arc) return;
  const target = arc.getAttribute("data-offset") ?? "0";
  requestAnimationFrame(() => {
    arc.style.strokeDashoffset = target;
  });
}

function renderStorageLegendMini(b: StorageBreakdown) {
  const items: [string, string, number][] = [
    ["Apps", "#ef4444", b.apps],
    ["Photos", "#eab308", b.photos],
    ["System", "#9ca3af", b.system],
  ];
  return items
    .filter(([, , val]) => val > 0)
    .map(
      ([label, color]) =>
        `<span class="legend-mini-item"><span class="legend-mini-dot" style="background:${color}"></span>${label}</span>`,
    )
    .join("");
}

function renderVerificationScoreRing(score: number) {
  const size = 48;
  const stroke = 5;
  const r = (size - stroke) / 2;
  const circ = 2 * Math.PI * r;
  const cx = size / 2;
  const offset = circ - (Math.min(100, Math.max(0, score)) / 100) * circ;

  return `
    <svg viewBox="0 0 ${size} ${size}">
      <circle cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke="#dcfce7" stroke-width="${stroke}"/>
      <circle cx="${cx}" cy="${cx}" r="${r}" fill="none" stroke="#22c55e" stroke-width="${stroke}"
        stroke-linecap="round" stroke-dasharray="${circ}" stroke-dashoffset="${circ}" data-offset="${offset}" class="score-arc"/>
    </svg>
    <span class="verification-score-overlay">${score}</span>`;
}

function animateScoreRing(container: HTMLElement) {
  const arc = container.querySelector(".score-arc") as SVGCircleElement | null;
  if (!arc) return;
  const target = arc.getAttribute("data-offset") ?? "0";
  requestAnimationFrame(() => {
    arc.style.strokeDashoffset = target;
  });
}

function pulseStatusCards() {
  document.querySelectorAll(".status-card").forEach((card) => {
    card.classList.remove("animate-in");
    void (card as HTMLElement).offsetWidth;
    card.classList.add("animate-in");
  });
}

const BATTERY_ANIMATION_MS = 1000;
const MODAL_MS_PER_PERCENT = 22;

function modalSegAnimation(delayMs: number, durationMs: number) {
  const delay = delayMs / 1000;
  const duration = durationMs / 1000;
  return `seg-grow-modal ${duration}s linear ${delay}s backwards`;
}

function renderStorageBar(b: StorageBreakdown, sequential = false) {
  if (!b.total) return "";
  const segments = [
    { label: "Apps", color: "#ef4444", val: b.apps },
    { label: "Photos", color: "#eab308", val: b.photos },
    { label: "Audio & Video", color: "#3b82f6", val: b.audio + b.videos },
    { label: "System", color: "#9ca3af", val: b.system },
    { label: "Other & Downloads", color: "#a855f7", val: b.other + b.downloads },
  ];
  let left = 0;
  let delayMs = 0;
  return segments
    .filter((s) => s.val > 0)
    .map((s) => {
      const width = (s.val / b.total) * 100;
      const durationMs = width * MODAL_MS_PER_PERCENT;
      const segDelayMs = delayMs;
      delayMs += durationMs;
      const pos = sequential ? `left:${left}%;width:${width}%;` : `width:${width}%;`;
      const anim = sequential
        ? `animation:${modalSegAnimation(segDelayMs, durationMs)};`
        : "";
      left += width;
      return `<div class="storage-seg" data-delay="${segDelayMs}" data-duration="${durationMs}" data-label="${escapeHtml(s.label)}" data-size="${escapeHtml(formatBytes(s.val))}" data-pct="${width.toFixed(1)}" style="${pos}background:${s.color};${anim}"></div>`;
    })
    .join("");
}

function renderStorageLegend(b: StorageBreakdown) {
  const items: [string, string, number][] = [
    ["System", "#9ca3af", b.system],
    ["Apps", "#ef4444", b.apps],
    ["Photos", "#eab308", b.photos],
    ["Audio & Video", "#3b82f6", b.audio + b.videos],
    ["Downloads", "#8b5cf6", b.downloads],
    ["Other", "#a855f7", b.other],
  ];
  return `<div class="storage-legend">${items
    .filter(([, , v]) => v > 0)
    .map(
      ([label, color, val]) =>
        `<span class="legend-item"><span class="legend-dot" style="background:${color}"></span>${label}: ${formatBytes(val)}</span>`,
    )
    .join("")}</div>`;
}

function renderDetailSections(sections: DetailSection[], maskLabels = false) {
  const sensitive = /serial|imei|address|id/i;
  return sections
    .map(
      (sec) => `
    <section class="detail-section">
      <h3>${escapeHtml(sec.title)}</h3>
      <div class="modal-grid">
        ${sec.rows
          .map((r) => {
            const val =
              maskLabels && sensitive.test(r.label) ? mask(r.value) : r.value;
            return `<div class="spec-row"><span>${escapeHtml(r.label)}</span><span>${escapeHtml(val)}${
              r.note
                ? `<span class="detail-note" title="${escapeHtml(r.note)}">${escapeHtml(r.note)}</span>`
                : ""
            }</span></div>`;
          })
          .join("")}
      </div>
    </section>`,
    )
    .join("");
}

function resultClass(result: string): string {
  if (result === "Normal") return "result-normal";
  if (result === "N/A") return "result-na";
  if (result === "Modified" || result === "Degraded" || result === "Restricted")
    return "result-warn";
  return "result-bad";
}

function renderVerificationTable(checks: VerificationCheck[]) {
  return `
    <table class="verification-table">
      <thead>
        <tr><th>Test Item</th><th>Ex-factory</th><th>Read Value</th><th>Result</th></tr>
      </thead>
      <tbody>
        ${checks
          .map(
            (c) => `
          <tr>
            <td>${escapeHtml(c.item)}</td>
            <td>${escapeHtml(c.factoryValue ?? "—")}</td>
            <td>${escapeHtml(mask(c.readValue))}</td>
            <td class="${resultClass(c.result)}">${escapeHtml(c.result)}${
              c.note
                ? `<div class="detail-note">${escapeHtml(c.note)}</div>`
                : ""
            }</td>
          </tr>`,
          )
          .join("")}
      </tbody>
    </table>`;
}

function renderSpecs(s: DeviceSummary) {
  const rows = [
    ["OS Version", `Android ${s.androidVersion}`],
    ["Security Patch", s.securityPatch],
    ["Serial Number", mask(s.serial)],
    ["IMEI", mask(s.imei)],
    ["IMEI 2", mask(s.imei2)],
    ["Model", s.model],
    ["Product", s.product],
    ["Sales Region", s.region],
    ["Activation", s.activationStatus],
    ["Root Status", s.rootStatus],
    ["FRP / Google Lock", s.frpStatus],
    ["Bootloader", s.bootloaderStatus],
    ["Manufacturing Date", s.manufacturingDate],
  ];
  $("spec-rows").innerHTML = rows
    .map(
      ([label, value]) =>
        `<div class="spec-row"><span>${label}</span><span>${escapeHtml(value)}</span></div>`,
    )
    .join("");
}

function replayStorageBarAnimation(container: ParentNode) {
  container.querySelectorAll(".storage-seg").forEach((seg) => {
    const el = seg as HTMLElement;
    const delayMs = Number(el.dataset.delay ?? 0);
    const durationMs = Number(el.dataset.duration ?? MODAL_MS_PER_PERCENT);
    const anim = modalSegAnimation(delayMs, durationMs);
    el.style.animation = "none";
    void el.offsetWidth;
    el.style.animation = anim;
  });
}

function initAppTooltips() {
  const tip = document.createElement("div");
  tip.className = "app-tooltip hidden";
  tip.setAttribute("role", "tooltip");
  document.body.appendChild(tip);

  let activeEl: HTMLElement | null = null;

  const mountTip = (el: HTMLElement) => {
    const dialog = el.closest("dialog");
    const host = dialog ?? document.body;
    if (tip.parentElement !== host) host.appendChild(tip);
    tip.style.zIndex = dialog ? "10" : "10000";
  };

  const position = (el: HTMLElement) => {
    const rect = el.getBoundingClientRect();
    const gap = 6;
    const margin = 8;
    const centerX = rect.left + rect.width / 2;

    tip.classList.remove("app-tooltip--below");
    tip.style.left = `${centerX}px`;
    tip.style.top = `${rect.top - gap}px`;

    const tipHeight = tip.offsetHeight;
    const showBelow = rect.top - gap - tipHeight < margin;

    if (showBelow) {
      tip.classList.add("app-tooltip--below");
      tip.style.top = `${rect.bottom + gap}px`;
    }

    const tipWidth = tip.offsetWidth;
    let left = centerX;
    const halfW = tipWidth / 2;
    if (left - halfW < margin) left = margin + halfW;
    if (left + halfW > window.innerWidth - margin) {
      left = window.innerWidth - margin - halfW;
    }
    tip.style.left = `${left}px`;
  };

  const tooltipText = (el: HTMLElement): string | null => {
    if (el.classList.contains("storage-seg") && el.dataset.label) {
      return `${el.dataset.label} · ${el.dataset.size} (${el.dataset.pct}%)`;
    }
    return null;
  };

  const show = (el: HTMLElement) => {
    const text = tooltipText(el);
    if (!text) return;
    activeEl = el;
    mountTip(el);
    tip.textContent = text;
    tip.classList.remove("hidden");
    position(el);
  };

  const hide = () => {
    activeEl = null;
    tip.classList.add("hidden");
    if (tip.parentElement !== document.body) document.body.appendChild(tip);
  };

  document.body.addEventListener("mouseover", (e) => {
    const el = (e.target as Element).closest(".storage-seg") as HTMLElement | null;
    if (el && tooltipText(el)) show(el);
  });

  document.body.addEventListener("mouseout", (e) => {
    const from = (e.target as Element).closest(".storage-seg");
    const to = (e.relatedTarget as Element | null)?.closest?.(".storage-seg");
    if (from && from !== to) hide();
  });

  window.addEventListener(
    "scroll",
    () => {
      if (activeEl) position(activeEl);
    },
    true,
  );
}

function openModal(id: string) {
  const dlg = document.getElementById(id) as HTMLDialogElement;
  dlg?.showModal();
}

function closeModals() {
  document.querySelector(".app-tooltip")?.classList.add("hidden");
  document.querySelectorAll(".modal").forEach((d) => {
    (d as HTMLDialogElement).close();
  });
}

function fillModals(s: DeviceSummary) {
  $("modal-device-body").innerHTML = renderDetailSections(
    s.deviceDetails,
    true,
  );

  $("modal-battery-body").innerHTML = `
    <div class="battery-modal-layout">
      <div class="battery-side">
        ${renderBatteryModalVisual(s)}
        <p class="muted">${escapeHtml(s.batteryChargingPower)}</p>
        <p class="muted">${escapeHtml(s.batteryTemperature)}</p>
      </div>
      <div>${renderDetailSections(s.batteryDetails)}</div>
    </div>`;

  const b = s.storageBreakdown;
  const storageUsed = formatBytes(b.total - b.free);
  $("modal-storage-body").innerHTML = `
    <p class="stat-big">${storageUsed}</p>
    <p class="muted">used of ${s.storageTotal}</p>
    <div class="storage-bar storage-bar--modal">${renderStorageBar(b, true)}</div>
    ${renderStorageLegend(b)}
    ${renderDetailSections(s.storageDetails)}`;

  $("modal-verification-body").innerHTML = `
    <p class="stat-ok">${escapeHtml(s.verificationStatus)}</p>
    <p class="muted">Check Score: ${s.verificationScore}</p>
    ${renderVerificationTable(s.verificationChecks)}
    <p class="modal-footnote">Su Android non esiste un protocollo factory come il lockdown iOS: molti campi mostrano solo il valore letto, senza confronto ex-factory. I componenti con SN (camera, display, batteria) richiedono API OEM o root.</p>`;
}

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i > 1 ? 2 : 0)} ${units[i]}`;
}

function renderSummary(s: DeviceSummary) {
  currentSummary = s;
  $("device-name").textContent = s.deviceName;
  const charge = s.isCharging ? s.chargingStatus : "Not Charging";
  $("device-subtitle").textContent = `${s.brand} ${s.model} · ${s.storageTotal} · ${charge} ${s.batteryLevel}%`;

  renderSpecs(s);

  const batteryGauge = $("battery-gauge");
  batteryGauge.innerHTML = renderGauge(s.batteryHealth);

  $("battery-subtitle").textContent = s.batteryTechnology || "Battery";
  $("charge-cycles").textContent =
    s.chargeCycles > 0 ? String(s.chargeCycles) : "N/A";

  const levelPill = $("battery-level-pill");
  levelPill.textContent = `${s.batteryLevel}% · ${s.isCharging ? "Charging" : "Level"}`;
  levelPill.classList.toggle("charging", s.isCharging);

  const b = s.storageBreakdown;
  const usedPct =
    b.total > 0 ? ((b.total - b.free) / b.total) * 100 : 0;
  const donut = $("storage-donut");
  donut.innerHTML = renderStorageDonut(usedPct);
  animateDonutArc(donut);

  $("storage-used-pct").textContent =
    b.total > 0 ? `${Math.round(usedPct)}% used` : "—";
  $("storage-free").textContent = formatBytes(b.total - b.free);
  $("storage-total").textContent = `used of ${s.storageTotal}`;

  const bar = $("storage-bar");
  if (b.total > 0) {
    bar.innerHTML = renderStorageBar(b);
    bar.classList.remove("hidden");
    $("storage-legend-mini").innerHTML = renderStorageLegendMini(b);
  } else {
    bar.classList.add("hidden");
    $("storage-legend-mini").innerHTML = "";
  }

  const scoreRing = $("verification-score-ring");
  scoreRing.innerHTML = renderVerificationScoreRing(s.verificationScore);
  animateScoreRing(scoreRing);

  fillModals(s);

  pulseStatusCards();
  showError("");
  updateScreens();
}

function resetMirrorUi() {
  mirrorImg.classList.add("hidden");
  mirrorImg.removeAttribute("src");
  phoneScreen.classList.remove("mirror-live");
  mirrorDeviceWidth = 0;
  mirrorDeviceHeight = 0;
}

async function stopMirrorPreview() {
  try {
    await invoke("stop_mirror_preview");
  } catch {
    /* ignore */
  }
  resetMirrorUi();
}

async function startMirrorPreview() {
  if (!selectedSerial) return;
  const device = devices.find((d) => d.serial === selectedSerial);
  if (!device || !isAuthorized(device)) return;

  try {
    await invoke("start_mirror_preview", { serial: selectedSerial });
  } catch (e) {
    showError(String(e));
  }
}

function onMirrorFrame(frame: MirrorFrame) {
  if (frame.serial !== selectedSerial) return;
  mirrorImg.src = frame.imageDataUrl;
  mirrorImg.classList.remove("hidden");
  phoneScreen.classList.add("mirror-live");
  mirrorDeviceWidth = frame.width;
  mirrorDeviceHeight = frame.height;
}

async function refreshSummary() {
  if (
    !selectedSerial ||
    !isAuthorized(devices.find((d) => d.serial === selectedSerial)!)
  ) {
    await stopMirrorPreview();
    currentSummary = null;
    updateScreens();
    return;
  }
  if (loading) return;
  loading = true;
  $("refresh-icon").classList.add("spinning");
  try {
    const summary = await invoke<DeviceSummary>("load_device_summary", {
      serial: selectedSerial,
    });
    renderSummary(summary);
  } catch (e) {
    await stopMirrorPreview();
    currentSummary = null;
    showError(String(e));
    screenDashboard.classList.add("hidden");
    screenEmpty.classList.remove("hidden");
  } finally {
    loading = false;
    $("refresh-icon").classList.remove("spinning");
  }
}

async function loadDevices() {
  try {
    devices = await invoke<AdbDevice[]>("list_devices");
    await applyDeviceList();
  } catch (e) {
    showError(String(e));
  }
}

async function handleBackupRestore() {
  try {
    const status = await invoke<{ installed: boolean }>("check_autobackup");
    if (!status.installed) {
      const install = await confirm(
        "AutoBackup non è installato. Vuoi scaricarlo da GitHub e installarlo ora?",
        { title: "Installa AutoBackup", kind: "info", okLabel: "Installa", cancelLabel: "Annulla" },
      );
      if (!install) return;
      await invoke("install_autobackup");
    }
    await invoke("launch_autobackup");
  } catch (e) {
    showError(String(e));
  }
}

async function handleAppManager() {
  try {
    const status = await invoke<{ installed: boolean }>("check_app_manager");
    if (!status.installed) {
      const install = await confirm(
        "AndroidAdwareCleaner non è installato. Vuoi scaricarlo da GitHub e installarlo ora?",
        { title: "Installa AppManager", kind: "info", okLabel: "Installa", cancelLabel: "Annulla" },
      );
      if (!install) return;
      await invoke("install_app_manager");
    }
    await invoke("launch_app_manager");
  } catch (e) {
    showError(String(e));
  }
}

let pendingUpdate: Update | null = null;
let continueAfterUpdateCheck: (() => void) | null = null;

function closeUpdateDialog() {
  const dlg = document.getElementById("modal-update") as HTMLDialogElement | null;
  dlg?.close();
  $("update-progress").classList.add("hidden");
  ($("btn-update-now") as HTMLButtonElement).disabled = false;
  ($("btn-update-later") as HTMLButtonElement).disabled = false;
  continueAfterUpdateCheck?.();
  continueAfterUpdateCheck = null;
}

function showUpdateDialog(update: Update) {
  pendingUpdate = update;
  $("update-version").textContent = `Versione ${update.version} disponibile`;
  $("update-notes").textContent =
    update.body?.trim() || "Sono disponibili miglioramenti e correzioni.";
  openModal("modal-update");
}

async function checkForAppUpdatesOnStartup() {
  try {
    const update = await check();
    if (!update) return;
    showUpdateDialog(update);
    await new Promise<void>((resolve) => {
      continueAfterUpdateCheck = resolve;
    });
  } catch {
    // offline o release non ancora pubblicata su GitHub
  }
}

async function checkForAppUpdates(silent = true) {
  try {
    const update = await check();
    if (!update) {
      if (!silent) showError("Sei già aggiornato.");
      return;
    }
    showUpdateDialog(update);
  } catch {
    if (!silent) {
      showError(
        "Controllo aggiornamenti non disponibile (offline o release non ancora pubblicata).",
      );
    }
  }
}

async function installPendingUpdate() {
  if (!pendingUpdate) return;
  const update = pendingUpdate;
  ($("btn-update-now") as HTMLButtonElement).disabled = true;
  ($("btn-update-later") as HTMLButtonElement).disabled = true;
  $("update-progress").classList.remove("hidden");

  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Progress") {
        const mb = (event.data.chunkLength / (1024 * 1024)).toFixed(1);
        $("update-progress-text").textContent = `Download in corso… (+${mb} MB)`;
      }
    });
    await relaunch();
  } catch (e) {
    showError(`Aggiornamento fallito: ${e}`);
    closeUpdateDialog();
  }
}

async function init() {
  hideSerialCheck.addEventListener("change", () => {
    if (currentSummary) renderSummary(currentSummary);
  });

  $("btn-refresh").addEventListener("click", () => void refreshSummary());
  $("btn-reboot").addEventListener("click", async () => {
    if (!selectedSerial) return;
    await stopMirrorPreview();
    await invoke("reboot_device", { serial: selectedSerial });
  });
  $("btn-shutdown").addEventListener("click", async () => {
    if (!selectedSerial) return;
    await stopMirrorPreview();
    await invoke("shutdown_device", { serial: selectedSerial });
  });

  phoneScreen.addEventListener("click", async (e) => {
    if (!selectedSerial || !mirrorDeviceWidth || !mirrorDeviceHeight) return;
    const rect = phoneScreen.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    try {
      await invoke("mirror_tap", {
        serial: selectedSerial,
        x,
        y,
        displayWidth: rect.width,
        displayHeight: rect.height,
        deviceWidth: mirrorDeviceWidth,
        deviceHeight: mirrorDeviceHeight,
      });
    } catch {
      /* tap best-effort */
    }
  });

  $("btn-device-details").addEventListener("click", () =>
    openModal("modal-device"),
  );
  $("card-battery").addEventListener("click", () => {
    replayBatteryModalAnimation($("modal-battery-body"));
    openModal("modal-battery");
  });
  $("card-storage").addEventListener("click", () => {
    replayStorageBarAnimation($("modal-storage-body"));
    openModal("modal-storage");
  });
  $("card-verification").addEventListener("click", () =>
    openModal("modal-verification"),
  );

  document.querySelectorAll("[data-close]").forEach((btn) => {
    btn.addEventListener("click", closeModals);
  });

  document.querySelectorAll("[data-close-update]").forEach((btn) => {
    btn.addEventListener("click", closeUpdateDialog);
  });

  $("btn-update-later").addEventListener("click", closeUpdateDialog);
  $("btn-update-now").addEventListener("click", () => void installPendingUpdate());
  $("btn-check-update").addEventListener("click", () => void checkForAppUpdates(false));

  const updateDlg = document.getElementById("modal-update");
  updateDlg?.addEventListener("click", (e) => {
    if (e.target === updateDlg) closeUpdateDialog();
  });

  document.querySelectorAll(".modal").forEach((dlg) => {
    dlg.addEventListener("click", (e) => {
      if (e.target === dlg) closeModals();
    });
  });

  document.querySelectorAll(".quick-action").forEach((btn) => {
    btn.addEventListener("click", () => {
      const action = (btn as HTMLElement).dataset.action;
      if (action === "backup") void handleBackupRestore();
      else if (action === "app-manager") void handleAppManager();
    });
  });

  initAppTooltips();

  await checkForAppUpdatesOnStartup();

  await listen<MirrorFrame>("mirror-frame", (ev) => onMirrorFrame(ev.payload));

  await listen<AdbDevice[]>("devices-changed", async (ev) => {
    devices = ev.payload;
    await applyDeviceList();
  });

  try {
    await invoke("get_adb_status");
  } catch (e) {
    showError(String(e));
  }

  await loadDevices();
  updateScreens();
}

void init();
