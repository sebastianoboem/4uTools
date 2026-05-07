# AutoBackup Portable

Un'applicazione portatile per il backup dei dati sviluppata in Python, progettata per essere semplice, efficace e senza installazione.

## Funzionalità

- **Wizard Intuitivo**: Guida passo-passo per la configurazione del backup.
- **Selezione Intelligente**: Rilevamento automatico delle unità USB.
- **Filtri Personalizzabili**: Seleziona categorie (Documenti, Foto, Video) o estensioni specifiche.
- **Esclusioni Sicure**: Ignora automaticamente cartelle di sistema (Windows, Program Files) e permette esclusioni manuali.
- **Backup Organizzato**: Mantiene la struttura originale delle cartelle all'interno di una suddivisione per categorie.
- **Verifica Integrità**: Controllo Hash MD5 opzionale per garantire la copia corretta.
- **Gestione Processo**: Pausa, Ripresa e Stop durante la copia.

---

## Installazione

### Pre-requisiti

- Windows 10/11 o macOS 12+
- Python 3.8+ (solo per esecuzione da sorgente)

### Da Sorgente

1. Clona il repository:
   ```bash
   git clone https://github.com/sebastianoboem/AutoBackup.git
   cd AutoBackup
   ```

2. Crea e attiva l'ambiente virtuale:
   ```bash
   python -m venv venv
   # macOS/Linux
   source venv/bin/activate
   # Windows
   venv\Scripts\activate
   ```

3. Installa le dipendenze:
   ```bash
   pip install -r requirements.txt
   ```

4. Installa Gum (CLI UI):
   - **macOS**:
     ```bash
     brew install gum
     ```
   - **Windows**: Già incluso nella cartella (`gum.exe`)

5. Esegui l'applicazione:
   ```bash
   python main.py
   ```

### Installazione Automatica ADB

L'applicazione controlla automaticamente la presenza di ADB all'avvio:

- **Windows**: Propone installazione via `winget` se mancante
- **macOS**: Propone installazione di Homebrew e poi ADB se mancanti
- L'utente viene informato se è necessario un riavvio del sistema

---

## Utilizzo

### Modalità Backup

L'applicazione offre due modalità di backup:

1. **Backup PC/Mac**: Backup dei file dal computer verso un'unità USB
2. **Backup Android**: Backup diretto da dispositivo Android via ADB

### Backup da Android

L'applicazione supporta il backup diretto da dispositivi Android tramite ADB.

#### Configurazione Android

1. **Abilitare Opzioni Sviluppatore**: Vai su *Impostazioni > Info telefono* e tocca 7 volte su *Numero build*.
2. **Attivare Debug USB**: Vai su *Impostazioni > Sistema > Opzioni sviluppatore* e attiva *Debug USB*.
3. **Collegamento**: Collega il telefono al PC via USB.
4. **Autorizzazione**: Sul display del telefono apparirà una richiesta "Consenti debug USB?". Seleziona "Consenti sempre..." e conferma.

#### Requisiti PC/Mac

- L'applicazione verifica automaticamente la presenza di ADB
- Se mancante, propone l'installazione automatica (winget su Windows, Homebrew su macOS)
- Alternativamente, posiziona l'eseguibile `adb` nella stessa cartella di `AutoBackup`

All'avvio dell'applicazione, seleziona la modalità **Backup da Android** e segui la procedura guidata.

---

## Struttura del Progetto

```
AutoBackup/
├── main.py                   # Interfaccia CLI interattiva con Gum
├── backup_engine.py          # Logica core (scansione, copia, verifica)
├── requirements.txt          # Dipendenze Python
├── build_exe.py              # Script build Windows (PyInstaller)
├── build_mac.py              # Script build macOS (PyInstaller)
├── tests/                    # Test automatizzati (pytest)
│   ├── test_backup_engine.py
│   ├── test_android_engine.py
│   └── test_main_app.py
├── launchd/                  # Template schedulazione macOS
│   └── com.autobackup.agent.plist
└── gum.exe / gum            # CLI UI (Windows/macOS)
```

---

## Contribuire al Progetto

### Ambiente di Sviluppo

1. Clona il repository:
   ```bash
   git clone https://github.com/sebastianoboem/AutoBackup.git
   cd AutoBackup
   ```

2. Crea il branch `dev`:
   ```bash
   git checkout -b dev origin/dev
   ```

3. Crea l'ambiente virtuale e installa le dipendenze:
   ```bash
   python -m venv venv
   source venv/bin/activate  # macOS/Linux
   # venv\Scripts\activate    # Windows
   pip install -r requirements.txt
   pip install pytest pytest-mock pytest-cov
   ```

4. Esegui i test:
   ```bash
   pytest
   ```

5. Esegui i test con coverage:
   ```bash
   pytest --cov=. --cov-report=term-missing
   ```

### Workflow di Sviluppo

1. Crea un branch dalla versione corrente:
   ```bash
   git checkout -b feature/nome-feature
   ```

2. Sviluppa la funzionalità e aggiungi test

3. Esegui i test e verifica la coverage:
   ```bash
   pytest --cov=. --cov-report=html
   ```

4. Commit le modifiche con messaggi descrittivi

5. Pusha il branch e crea una Pull Request verso `dev`

6. Dopo review, il codice verrà mergiato su `dev` e incluso nella prossima release

### Versionamento

Il progetto usa **Semantic Versioning** (Major.Minor.Bugfix):

- **Major**: Cambiamenti incompatibili API o rework completo
- **Minor**: Nuove funzionalità retro-compatibili
- **Bugfix**: Correzioni di bug retro-compatibili

Le release sono taggate con il formato `vX.Y.Z` (es. `v1.1.0`)

---

## Schedulazione Automatica (macOS)

Per eseguire il backup in automatico su macOS:

1. Copia il file `launchd/com.autobackup.agent.plist` in `~/Library/LaunchAgents/` e modifica `USERNAME` e i percorsi.
2. Carica l'agente:
   ```bash
   launchctl load -w ~/Library/LaunchAgents/com.autobackup.agent.plist
   ```
3. Log disponibili in `~/Library/Logs/AutoBackup/`.