## Obiettivo
Creare una versione macOS del tool di backup interattivo, con le stesse funzionalità della versione Windows: selezione categorie/estensioni, whitelist/esclusioni, scelta del disco di destinazione (USB), scansione e copia verificata, logging degli errori, e opzione di schedulazione automatica.

## Analisi del codice attuale
- Interfaccia: `main_gum.py` usa Gum per l’UI interattiva (rilevamento binario a `main_gum.py:26-42`, import tardivo di `shutil` a `main_gum.py:552`).
- Scansione/schedulazione: non c’è integrazione di scheduler; tutto è avviato manualmente.
- Rilevamento dischi fissi: controlla `psutil.disk_partitions()` cercando `fixed` (`main_gum.py:337-346`).
- Destinazione USB: `backup_engine.get_removable_drives()` cerca `removable` in `opts` (`backup_engine.py:32-48`) — non funziona su macOS.
- Limitazione system drive: logica specifica per Windows (`backup_engine.py:82-113`).
- Struttura di destinazione: usa lettera di drive (`src.drive`) per il path (`backup_engine.py:232-240`) — su macOS è vuota.
- Esclusioni di sistema: lista Windows (`backup_engine.py:25-30`).

## Modifiche cross‑platform (macOS)
- Gum cross‑platform:
  - Import di `shutil` all’inizio e detection del binario con `shutil.which('gum')` (fallback al binario locale/impacchettato). Aggiornare messaggi: non forzare `gum.exe` su macOS (`main_gum.py:26-42`).
- Rilevamento destinazione (USB/volumi):
  - Implementare detection macOS: elencare partizioni con `psutil.disk_partitions()` filtrando `mountpoint` che inizia con `/Volumes/`, con `psutil.disk_usage` per spazio libero; se la lista è vuota, fallback su `os.listdir('/Volumes')` costruendo i mountpoint. Aggiornare `backup_engine.get_removable_drives()` (`backup_engine.py:32-48`).
- Sorgenti da scansionare:
  - Su macOS, default alla home utente (`os.path.expanduser('~')`), già preimpostata in `GumBackupApp.whitelist_paths` (`main_gum.py:24-25`).
  - Sostituire la ricerca di "dischi fissi" basata su `fixed` con una strategia: se whitelist è vuota → usare solo la home; se presente → usare i percorsi della whitelist. Adattare `main_gum.py:337-386` per branch macOS.
- Esclusioni di sistema (macOS):
  - Aggiungere set dedicato: `['System','Library','Applications','private','usr','var','.Spotlight-V100','.fseventsd','Volumes']` e usare questo quando `platform.system()=='Darwin'` (`backup_engine.py:25-30`).
- Struttura di destinazione coerente:
  - Quando si scansiona un root `/Volumes/<Nome>`, derivare `volume_label='<Nome>'` e usarla al posto di `drive_letter` in `copy_files`. Possibile approccio: aggiungere a ogni item `source_root_label` durante `scan_files` (in base al root `drive_path`) e usare quello per comporre `Dest/Categoria/<label>/...` (`backup_engine.py:224-240`).
- Verifica e logging:
  - Invariati: md5 completo (`backup_engine.py:261-276`) e `backup_errors.log` (`main_gum.py:529-533`).

## Packaging macOS
- PyInstaller: aggiungere uno script di build per macOS (one‑file) che includa il binario `gum` se disponibile in PATH. Opzioni: `--onefile`, `--add-binary gum:<dir>` (solo se rilevato). Adattare `build_exe.py` o creare `build_mac.py`.
- Requisiti: assicurare `psutil`, `colorama`, `tqdm` nel `requirements.txt`.

## Schedulazione automatica (facoltativa)
- Fornire un `LaunchAgent` plist di esempio: `~/Library/LaunchAgents/com.autobackup.agent.plist` che esegue lo script a orario prestabilito (es. quotidiano alle 20:00), con `StandardOutPath/StandardErrorPath` verso una cartella log.
- Istruzioni: `launchctl load -w ~/Library/LaunchAgents/com.autobackup.agent.plist`, modificare `ProgramArguments` con il path dello script impacchettato o `python3 main_gum.py`.

## Documentazione
- Aggiornare `README.md` con sezione macOS:
  - Installazione: `brew install gum`, `python3 -m venv .venv && source .venv/bin/activate && pip install -r requirements.txt`.
  - Esecuzione: `python3 main_gum.py`.
  - Selezione volume USB: appare l’elenco in `/Volumes`.
  - Schedulazione con LaunchAgents.

## Verifica
- Test manuali:
  - Senza gum: verificare messaggio d’errore chiaro e fallback.
  - Con gum: selezione categorie, whitelist home, esclusioni macOS, scelta volume USB, copia e verifica md5.
- Test edge:
  - Volumi con spazi/UTF‑8 nel nome; permessi negati; file grandi.
- Output: struttura `Dest/Backup_<timestamp>/<Categoria>/<VolumeLabel>/<path_relativo>`.

## Consegne
- Codice aggiornato cross‑platform (senza duplicare file per macOS).
- Script di build macOS.
- Template LaunchAgent e istruzioni.
- README aggiornato con sezione macOS.

Confermi che proceda con queste modifiche?