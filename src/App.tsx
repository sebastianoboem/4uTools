import { useEffect, useMemo, useState } from "react";
import { PreviewByCategory } from "./components/PreviewByCategory";
import { api, BackupResult, CategoryInfo, CopyProgress, DriveInfo, FileEntry, ScanProgress, formatBytes } from "./lib/api";
import { prefetchImageThumbnails } from "./lib/previewCache";
import "./App.css";

type Mode = "pc" | "android";
type Step = 1 | 2 | 3 | 4 | 5 | 6 | 7;

function PcMacIcon() {
  return (
    <svg className="card-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="2" y="4" width="20" height="13" rx="2" ry="2" />
      <path d="M8 20h8M12 17v3" />
    </svg>
  );
}

function AndroidPhoneIcon() {
  return (
    <svg className="card-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="7" y="2" width="10" height="20" rx="2" ry="2" />
      <circle cx="12" cy="18" r="1" />
      <path d="M9 5h.01M15 5h.01" />
    </svg>
  );
}

function groupFilesByCategory(files: FileEntry[], order: string[]) {
  const map = new Map<string, FileEntry[]>();
  for (const f of files) {
    const list = map.get(f.category) ?? [];
    list.push(f);
    map.set(f.category, list);
  }

  const groups: { category: string; files: FileEntry[]; size: number }[] = [];
  const seen = new Set<string>();

  for (const cat of order) {
    const items = map.get(cat);
    if (items?.length) {
      groups.push({
        category: cat,
        files: items,
        size: items.reduce((sum, f) => sum + f.size, 0),
      });
      seen.add(cat);
    }
  }

  for (const [cat, items] of map) {
    if (!seen.has(cat)) {
      groups.push({
        category: cat,
        files: items,
        size: items.reduce((sum, f) => sum + f.size, 0),
      });
    }
  }

  return groups;
}

function App() {
  const [step, setStep] = useState<Step>(1);
  const [mode, setMode] = useState<Mode | null>(null);
  const [categories, setCategories] = useState<CategoryInfo[]>([]);
  const [categoryMap, setCategoryMap] = useState<Record<string, string[]>>({});
  const [whitelist, setWhitelist] = useState("");
  const [exclusions, setExclusions] = useState("");
  const [exceptions, setExceptions] = useState("");
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [destination, setDestination] = useState<DriveInfo | null>(null);
  const [customDest, setCustomDest] = useState("");
  const [destIsDriveRoot, setDestIsDriveRoot] = useState(false);
  const [deviceId, setDeviceId] = useState<string>("");
  const [adbPath, setAdbPath] = useState<string | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [selectedSources, setSelectedSources] = useState<Set<string>>(new Set());
  const [scanRoots, setScanRoots] = useState<string[]>([]);
  const [verify, setVerify] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [copyProgress, setCopyProgress] = useState<CopyProgress | null>(null);
  const [result, setResult] = useState<BackupResult | null>(null);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    (async () => {
      const cats = await api.getCategories();
      const deselected = await api.getDefaultDeselected();
      setCategories(cats);
      const defaults = ["Immagini", "Video", "Documenti"];
      const map: Record<string, string[]> = {};
      for (const c of cats) {
        if (defaults.includes(c.name)) {
          map[c.name] = c.extensions.filter((e) => !deselected.includes(e));
        } else {
          map[c.name] = [];
        }
      }
      setCategoryMap(map);
      const home = await api.getHomeDir();
      if (home) setWhitelist(home);
      setAdbPath(await api.checkAdb());
    })();
  }, []);

  useEffect(() => {
  const unsubs: Array<() => void> = [];
  (async () => {
    unsubs.push(await api.onScanProgress(setScanProgress));
    unsubs.push(await api.onBackupProgress(setCopyProgress));
    unsubs.push(await api.onBackupComplete(setResult));
  })();
  return () => unsubs.forEach((u) => u());
}, []);

  const destMount = destination?.mountpoint ?? customDest;

  async function loadDrives() {
    setDrives(await api.listDrives());
  }

  async function pickDestFolder() {
    const picked = await api.pickFolder();
    if (typeof picked === "string") {
      setCustomDest(picked);
      setDestIsDriveRoot(false);
      const info = await api.getDriveForPath(picked);
      if (info) setDestination(info);
    }
  }

  async function pickWhitelistFolder() {
    const picked = await api.pickFolder();
    if (typeof picked === "string") setWhitelist((w) => (w ? `${w}\n${picked}` : picked));
  }

  const hasActiveFilters = Object.values(categoryMap).some((exts) => exts.length > 0);

  useEffect(() => {
    setSelectedSources(new Set(files.map((f) => f.source)));
  }, [files]);

  const selectedFiles = useMemo(
    () => files.filter((f) => selectedSources.has(f.source)),
    [files, selectedSources],
  );

  const selectedTotalSize = useMemo(
    () => selectedFiles.reduce((sum, f) => sum + f.size, 0),
    [selectedFiles],
  );

  const previewGroups = useMemo(
    () => groupFilesByCategory(files, categories.map((c) => c.name)),
    [files, categories],
  );

  function isAllSelected(cat: CategoryInfo) {
    const selected = categoryMap[cat.name] ?? [];
    return selected.length > 0 && selected.length === cat.extensions.length;
  }

  function isSomeSelected(cat: CategoryInfo) {
    const selected = categoryMap[cat.name] ?? [];
    return selected.length > 0 && selected.length < cat.extensions.length;
  }

  function toggleCategory(cat: CategoryInfo) {
    setCategoryMap((m) => ({
      ...m,
      [cat.name]: isAllSelected(cat) ? [] : [...cat.extensions],
    }));
  }

  function toggleExtension(catName: string, ext: string) {
    setCategoryMap((m) => {
      const cur = m[catName] ?? [];
      const next = cur.includes(ext) ? cur.filter((e) => e !== ext) : [...cur, ext];
      return { ...m, [catName]: next };
    });
  }

  function toggleFileSelected(source: string, checked: boolean) {
    setSelectedSources((prev) => {
      const next = new Set(prev);
      if (checked) next.add(source);
      else next.delete(source);
      return next;
    });
  }

  async function goStep2() {
    if (!mode) return;
    setError("");
    if (mode === "android") {
      if (!adbPath) {
        setError("ADB non trovato. Installalo con: brew install android-platform-tools");
        return;
      }
      const devs = await api.listAndroidDevices();
      if (devs.length === 0) {
        setError("Nessun dispositivo Android trovato. Verifica debug USB e autorizzazione.");
        return;
      }
      setDeviceId(devs[0].id);
    }
    const excl = await api.getDefaultExclusions(mode);
    const exc = await api.getDefaultExceptions(mode);
    setExclusions(excl.join("\n"));
    setExceptions(exc.join("\n"));
    setStep(2);
  }

  async function runScan() {
    if (!mode) return;
    setBusy(true);
    setError("");
    setScanProgress(null);
    try {
      const res = await api.runScan({
        mode,
        category_map: Object.fromEntries(
          Object.entries(categoryMap).filter(([, exts]) => exts.length > 0),
        ),
        custom_extensions: [],
        exclusions: exclusions.split("\n").map((s) => s.trim()).filter(Boolean),
        exceptions: exceptions.split("\n").map((s) => s.trim()).filter(Boolean),
        whitelist_paths: whitelist.split("\n").map((s) => s.trim()).filter(Boolean),
        destination_mount: destMount,
        device_id: mode === "android" ? deviceId : undefined,
        test_mode: false,
      });
      setFiles(res.files);
      setScanRoots(res.scan_roots);
      if (mode === "android") {
        prefetchImageThumbnails(res.files);
      }
      setStep(5);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function runBackup() {
    if (!mode) return;
    setBusy(true);
    setError("");
    setCopyProgress(null);
    setStep(6);
    try {
      const res = await api.runBackup(mode, destMount, destIsDriveRoot, verify, selectedFiles);
      setResult(res);
      setStep(7);
    } catch (e) {
      setError(String(e));
      setStep(5);
    } finally {
      setBusy(false);
      setPaused(false);
    }
  }

  return (
    <div className="app">
      <main className={`content${step === 1 ? " content-centered" : ""}`}>
        {error && <div className="error">{error}</div>}

        {step === 1 && (
          <section className="mode-screen">
            <h2>Scegli modalità</h2>
            <div className="cards cards-centered">
              <button className={`card ${mode === "pc" ? "selected" : ""}`} onClick={() => setMode("pc")}>
                <strong>PC/Mac</strong>
                <PcMacIcon />
                <p>Copia file dal computer verso USB o cartella locale</p>
              </button>
              <button className={`card ${mode === "android" ? "selected" : ""}`} onClick={() => setMode("android")}>
                <strong>Android</strong>
                <AndroidPhoneIcon />
                <p>Copia file via ADB da dispositivo collegato</p>
              </button>
            </div>
            <div className="mode-confirm">
              <button className="primary" onClick={goStep2} disabled={!mode}>Conferma</button>
            </div>
          </section>
        )}

        {step === 2 && (
          <section>
            <h2>Filtri file</h2>
            <div className="actions">
              <button onClick={() => setStep(1)}>Indietro</button>
              <button className="primary" onClick={() => setStep(3)} disabled={!hasActiveFilters}>Avanti</button>
            </div>
            <div className="filter-grid">
              {categories.map((cat) => (
                <div key={cat.name} className="filter-category">
                  <label className="filter-category-name">
                    <input
                      type="checkbox"
                      ref={(el) => {
                        if (el) el.indeterminate = isSomeSelected(cat);
                      }}
                      checked={isAllSelected(cat)}
                      onChange={() => toggleCategory(cat)}
                    />
                    {cat.name}
                  </label>
                  <div className="filter-extensions">
                    {cat.extensions.map((ext) => (
                      <label key={ext} className="filter-ext">
                        <input
                          type="checkbox"
                          checked={categoryMap[cat.name]?.includes(ext) ?? false}
                          onChange={() => toggleExtension(cat.name, ext)}
                        />
                        {ext}
                      </label>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

        {step === 3 && (
          <section>
            <h2>Percorsi ed esclusioni</h2>
            <div className="actions">
              <button onClick={() => setStep(2)}>Indietro</button>
              <button className="primary" onClick={async () => { await loadDrives(); setStep(4); }}>Avanti</button>
            </div>
            <div className="paths-layout">
              {mode === "pc" && (
                <div className="paths-col">
                  <label className="field">
                    Whitelist (un percorso per riga, default: home)
                    <textarea value={whitelist} onChange={(e) => setWhitelist(e.target.value)} rows={8} />
                  </label>
                  <button onClick={pickWhitelistFolder}>Aggiungi cartella...</button>
                </div>
              )}
              <label className="field paths-col">
                Cartelle da escludere
                <textarea value={exclusions} onChange={(e) => setExclusions(e.target.value)} rows={8} />
              </label>
              <label className="field paths-col">
                Cartelle da includere (nonostante le esclusioni)
                <textarea value={exceptions} onChange={(e) => setExceptions(e.target.value)} rows={8} />
              </label>
            </div>
          </section>
        )}

        {step === 4 && (
          <section>
            <h2>Destinazione</h2>
            <div className="actions">
              <button onClick={() => setStep(3)}>Indietro</button>
              <button className="primary" onClick={runScan} disabled={!destMount || busy}>
                {busy ? "Scansione..." : "Scansiona"}
              </button>
            </div>
            <div className="dest-layout">
              <div className={`dest-col ${destIsDriveRoot && destination ? "selected" : ""}`}>
                <div className="dest-col-header">
                  <h3>Unità USB</h3>
                  <button type="button" className="dest-btn" onClick={loadDrives}>Aggiorna</button>
                </div>
                <div className="drive-list">
                  {drives.length === 0 ? (
                    <p className="hint">Nessuna unità USB rilevata</p>
                  ) : (
                    drives.map((d) => (
                      <button
                        key={d.mountpoint}
                        type="button"
                        className={`drive ${destIsDriveRoot && destination?.mountpoint === d.mountpoint ? "selected" : ""}`}
                        onClick={() => { setDestination(d); setCustomDest(""); setDestIsDriveRoot(true); }}
                      >
                        <strong>{d.mountpoint}</strong>
                        <span>{d.device}</span>
                        <span>{Math.floor(d.free / 1024 ** 3)} GB liberi</span>
                      </button>
                    ))
                  )}
                </div>
              </div>
              <div className={`dest-col ${!destIsDriveRoot && destMount ? "selected" : ""}`}>
                <h3>Cartella personalizzata</h3>
                <div className="dest-custom">
                  <input
                    value={customDest}
                    onChange={(e) => { setCustomDest(e.target.value); setDestIsDriveRoot(false); setDestination(null); }}
                    placeholder="Percorso cartella"
                  />
                  <button type="button" className="dest-btn" onClick={pickDestFolder}>Sfoglia...</button>
                </div>
                <p className="hint">Usa questa opzione se non hai un&apos;unità USB o vuoi una cartella specifica.</p>
              </div>
            </div>
            <label className="field inline">
              <input type="checkbox" checked={verify} onChange={(e) => setVerify(e.target.checked)} />
              Verifica integrità MD5 (solo PC)
            </label>
            {scanProgress && (
              <p className="hint">Scansione: {scanProgress.current_file} ({scanProgress.files_found} file)</p>
            )}
          </section>
        )}

        {step === 5 && (
          <section>
            <h2>Anteprima</h2>
            <div className="actions">
              <button onClick={() => setStep(4)}>Indietro</button>
              <button className="primary" onClick={runBackup} disabled={selectedFiles.length === 0 || busy}>
                Avvia backup
              </button>
            </div>
            <p>{selectedFiles.length} file selezionati — {formatBytes(selectedTotalSize)}</p>
            <p className="hint">Sorgente: {scanRoots.join(", ")}</p>
            <PreviewByCategory
              groups={previewGroups}
              mode={mode}
              selectedSources={selectedSources}
              onToggle={toggleFileSelected}
            />
          </section>
        )}

        {step === 6 && (
          <section>
            <h2>Backup in corso</h2>
            <div className="actions">
              {!paused ? (
                <button onClick={() => { api.pauseBackup(); setPaused(true); }}>Pausa</button>
              ) : (
                <button onClick={() => { api.resumeBackup(); setPaused(false); }}>Riprendi</button>
              )}
              <button className="danger" onClick={() => api.stopBackup()}>Stop</button>
            </div>
            {copyProgress && (
              <>
                <p>{copyProgress.current_file}</p>
                <div className="progress-bar">
                  <div
                    className="progress-fill"
                    style={{ width: `${copyProgress.total_bytes ? (copyProgress.copied_bytes / copyProgress.total_bytes) * 100 : 0}%` }}
                  />
                </div>
                <p>{copyProgress.copied_count} / {copyProgress.total_count} file</p>
              </>
            )}
          </section>
        )}

        {step === 7 && result && (
          <section>
            <h2>Completato</h2>
            <div className="actions">
              <button className="primary" onClick={() => api.openFolder(result.destination)}>Apri cartella</button>
              <button onClick={() => { setStep(1); setResult(null); setFiles([]); setMode(null); }}>Nuovo backup</button>
            </div>
            <p className="success">{result.copied_count} file copiati in:</p>
            <p><code>{result.destination}</code></p>
            {result.errors.length > 0 && (
              <p className="error">{result.errors.length} errori (vedi backup_errors.log)</p>
            )}
          </section>
        )}
      </main>
    </div>
  );
}

export default App;
