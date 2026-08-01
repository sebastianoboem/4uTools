import { useCallback, useEffect, useRef, useState } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { checkForAppUpdate, installAppUpdate, UpdateStatus } from "../lib/updater";

export function UpdateBanner() {
  const [status, setStatus] = useState<UpdateStatus>({ state: "idle" });
  const pendingRef = useRef<Update | null>(null);

  const runCheck = useCallback(async (manual = false) => {
    setStatus({ state: "checking" });
    try {
      const result = await checkForAppUpdate();
      if (!result.available) {
        pendingRef.current = null;
        setStatus(manual ? { state: "upToDate" } : { state: "idle" });
        return;
      }
      pendingRef.current = result.update;
      setStatus({
        state: "available",
        version: result.version,
        body: result.body,
      });
    } catch (e) {
      pendingRef.current = null;
      if (manual) {
        setStatus({ state: "error", message: String(e) });
      } else {
        setStatus({ state: "idle" });
      }
    }
  }, []);

  useEffect(() => {
    const t = window.setTimeout(() => {
      void runCheck(false);
    }, 1500);
    return () => window.clearTimeout(t);
  }, [runCheck]);

  async function onInstall() {
    const pending = pendingRef.current;
    if (!pending || status.state !== "available") return;
    const version = status.version;
    setStatus({ state: "downloading", version, progress: 0 });
    try {
      await installAppUpdate(pending, (downloaded, total) => {
        const progress = total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
        setStatus({ state: "downloading", version, progress });
      });
      setStatus({ state: "installing", version });
    } catch (e) {
      setStatus({ state: "error", message: String(e) });
    }
  }

  if (status.state === "idle") {
    return (
      <button type="button" className="update-check-btn" onClick={() => void runCheck(true)}>
        Controlla aggiornamenti
      </button>
    );
  }

  if (status.state === "checking") {
    return <div className="update-banner is-muted">Controllo aggiornamenti…</div>;
  }

  if (status.state === "upToDate") {
    return (
      <div className="update-banner is-ok">
        App aggiornata
        <button type="button" className="update-dismiss" onClick={() => setStatus({ state: "idle" })}>
          Chiudi
        </button>
      </div>
    );
  }

  if (status.state === "error") {
    return (
      <div className="update-banner is-error">
        Aggiornamento non riuscito: {status.message}
        <button type="button" className="update-dismiss" onClick={() => setStatus({ state: "idle" })}>
          Chiudi
        </button>
      </div>
    );
  }

  if (status.state === "downloading" || status.state === "installing") {
    return (
      <div className="update-banner">
        {status.state === "downloading"
          ? `Download v${status.version}… ${status.progress}%`
          : `Installazione v${status.version}…`}
      </div>
    );
  }

  return (
    <div className="update-banner">
      <div className="update-banner-text">
        <strong>Nuova versione {status.version}</strong>
        {status.body ? <span>{status.body}</span> : null}
      </div>
      <div className="update-banner-actions">
        <button type="button" className="primary" onClick={() => void onInstall()}>
          Aggiorna
        </button>
        <button type="button" onClick={() => setStatus({ state: "idle" })}>
          Più tardi
        </button>
      </div>
    </div>
  );
}
