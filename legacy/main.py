import os
import sys
import subprocess
import psutil
import time
import shutil
from backup_engine import BackupEngine, AndroidBackupEngine
from colorama import init, Fore, Style
from tqdm import tqdm

# Initialize Colorama
init(autoreset=True)

class GumBackupApp:
    def __init__(self):
        self.mode = "PC" # Default
        self.engine = BackupEngine()
        self.android_engine = None
        self.android_device_id = None
        self.selected_categories = []
        self.custom_extensions = []
        self.exclusions = []
        self.exceptions = []
        self.selected_drive = None
        self.gum_exe = self._find_gum()
        self.test_mode = False
        self.active_category_map = {}
        self.extra_folders = []
        self.whitelist_paths = [os.path.expanduser("~")] # Default to user home

    def _find_gum(self):
        if hasattr(sys, '_MEIPASS'):
            path1 = os.path.join(sys._MEIPASS, "gum")
            path2 = os.path.join(sys._MEIPASS, "gum.exe")
            if os.path.exists(path1):
                return path1
            if os.path.exists(path2):
                return path2
        local_dir = os.path.dirname(os.path.abspath(__file__))
        local1 = os.path.join(local_dir, "gum")
        local2 = os.path.join(local_dir, "gum.exe")
        if os.path.exists(local1):
            return local1
        if os.name == 'nt' and os.path.exists(local2):
            return local2
        if shutil.which("gum"):
            return "gum"
        return None

    def _run_gum(self, args, input_text=None):
        if not self.gum_exe:
            print(Fore.RED + "Errore: gum non trovato! Assicurati che sia nella cartella del programma o nel PATH.")
            sys.exit(1)
            
        cmd = [self.gum_exe] + args
        try:
            kwargs = {
                'stdout': subprocess.PIPE,
                'text': True,
                'encoding': 'utf-8',
                'check': False
            }
            
            if input_text is not None:
                kwargs['input'] = input_text
            
            result = subprocess.run(cmd, **kwargs)
            return result
        except Exception as e:
            print(Fore.RED + f"Errore esecuzione gum: {e}")
            sys.exit(1)

    def clear_screen(self):
        if self.gum_exe:
             os.system('cls' if os.name == 'nt' else 'clear')
        else:
             os.system('cls' if os.name == 'nt' else 'clear')

    def print_header(self, title):
        self.clear_screen()
        if self.gum_exe:
            self._run_gum(["style", "--foreground", "212", "--border", "double", "--padding", "0 1", "--margin", "1", f"AutoBackup (Gum) - {title}"])
        else:
            print(Fore.MAGENTA + Style.BRIGHT + f"--- AutoBackup (Gum) - {title} ---")

    def step_select_mode(self):
        self.print_header("Seleziona Modalità")
        res = self._run_gum(["choose", "--header", "Che tipo di backup vuoi effettuare?", "Backup di PC/Mac", "Backup di Android"])
        
        if res.returncode != 0:
            sys.exit()
            
        if "Android" in res.stdout:
            self.mode = "Android"
            self.whitelist_paths = [] # Clear default whitelist for Android
            self.android_engine = AndroidBackupEngine()
            
            # Check ADB
            if not self.android_engine.adb_exe:
                print(Fore.RED + "ADB non trovato. Installalo o assicurati che sia nel PATH.")
                sys.exit(1)
                
            while True:
                devices = self.android_engine.get_devices()
                if not devices:
                    print(Fore.RED + "Nessun dispositivo Android trovato via ADB.")
                    print(Fore.YELLOW + "Assicurati che:")
                    print("1. Il debug USB sia attivo.")
                    print("2. Il telefono sia collegato.")
                    print("3. Hai autorizzato il computer sul telefono.")
                    res = self._run_gum(["confirm", "Riprovare?"])
                    if res.returncode == 0:
                        continue
                    else:
                        sys.exit()
                
                if len(devices) > 1:
                    # Select device
                    choices = [f"{d['id']} ({d['model']})" for d in devices]
                    res = self._run_gum(["choose", "--header", "Seleziona dispositivo"] + choices)
                    selected_id = res.stdout.strip().split()[0]
                    self.android_device_id = selected_id
                    break
                else:
                    self.android_device_id = devices[0]['id']
                    print(Fore.GREEN + f"Dispositivo rilevato: {devices[0]['model']} ({devices[0]['id']})")
                    time.sleep(1)
                    break
        else:
            self.mode = "PC"

    def step1_select_filters(self):
        self.print_header("Filtri File")
        
        cats = list(self.engine.categories.keys())
        default_selected = ["Documenti", "Immagini", "Video"]
        print(Fore.CYAN + "Seleziona le categorie da includere:")
        
        res = self._run_gum([
            "choose", 
            "--no-limit", 
            "--height", "15", 
            "--cursor=-> ", 
            "--selected", ",".join(default_selected)
        ] + cats)
        
        if res.returncode != 0:
            sys.exit()
            
        self.selected_categories = [x for x in res.stdout.strip().split('\n') if x]
        
        if not self.selected_categories:
            print(Fore.RED + "\nNessuna categoria selezionata.")
            time.sleep(1)
            sys.exit()

        self.active_category_map = {}
        default_deselected_exts = [
            ".txt", ".gif", ".bmp", ".tiff", ".raw", ".ico", 
            ".mkv", ".flv", ".flac", ".ogg", ".aac", ".tar", ".gz"
        ]
        
        for cat in self.selected_categories:
            self.clear_screen()
            exts = sorted(self.engine.categories[cat])
            if not exts: continue
            pre_selected = [e for e in exts if e not in default_deselected_exts]
            print(Fore.CYAN + f"Filtro Estensioni per {cat}:")
            print(Fore.CYAN + "Seleziona le estensioni che vuoi copiare")
            
            gum_args = [
                "choose", 
                "--no-limit", 
                "--height", "10", 
                "--cursor=-> ", 
                "--header=", 
            ]
            if pre_selected:
                gum_args.extend(["--selected", ",".join(pre_selected)])
            gum_args.extend(exts)
            
            res = self._run_gum(gum_args)
            if res.returncode != 0: sys.exit()
            selected_exts_list = [x for x in res.stdout.strip().split('\n') if x]
            
            if selected_exts_list:
                self.active_category_map[cat] = selected_exts_list
            else:
                print(Fore.YELLOW + f"Nessuna estensione selezionata per {cat}. La categoria verrà ignorata.")
                time.sleep(1)

        if not self.active_category_map:
             print(Fore.RED + "\nNessuna categoria/estensione attiva selezionata.")
             time.sleep(2)
             sys.exit()

        self.clear_screen()
        res = self._run_gum(["confirm", "--default=false", "Vuoi aggiungere estensioni personalizzate?"])
        if res.returncode == 0:
            res_input = self._run_gum(["input", "--placeholder", "es. .log, .dat"])
            exts = res_input.stdout.strip()
            if exts:
                self.custom_extensions = [e.strip() for e in exts.split(",") if e.strip()]

        self.clear_screen()
        
        # Only show Whitelist configuration if NOT Android
        if self.mode != "Android":
            print(Fore.CYAN + "Definizione Percorsi di Ricerca")
            print(Fore.CYAN + "Default: Ricerca automatica su tutti i dischi fissi.")
            print(Fore.CYAN + "Opzionale: Puoi specificare una lista di cartelle precise (Whitelist).")
            
            if self.whitelist_paths:
                 print(Fore.YELLOW + "\nWhitelist attuale:")
                 for p in self.whitelist_paths:
                     print(Fore.YELLOW + f"   - {p}")
            else:
                 print(Fore.YELLOW + "\nWhitelist attuale: (Nessuna - Scansione Completa)")
                 
            print("")
            res = self._run_gum(["confirm", "--default=false", "Vuoi modificare la lista delle cartelle esclusive?"])
            if res.returncode == 0:
                 print(Fore.CYAN + "\nInserisci i percorsi da scansionare (uno per riga).")
                 print(Fore.CYAN + "Premi CTRL+D (o CTRL+Z su Windows) poi INVIO per terminare.")
                 current_value = "\n".join(self.whitelist_paths) if self.whitelist_paths else ""
                 res_write = self._run_gum(["write", "--value", current_value, "--placeholder", "Esempio:\nC:\\Dati_Importanti\nD:\\Foto_Vacanze"])
                 
                 if res_write.returncode == 0:
                     paths = res_write.stdout.strip().split('\n')
                     self.whitelist_paths = []
                     for p in paths:
                         clean_p = p.strip()
                         if clean_p:
                             if os.path.exists(clean_p) and os.path.isdir(clean_p):
                                self.whitelist_paths.append(clean_p)
                             else:
                                print(Fore.YELLOW + f"Attenzione: '{clean_p}' non esiste o non è una cartella.")
                                time.sleep(1)
                                
                 if self.whitelist_paths:
                      print(Fore.GREEN + f"MODALITÀ WHITELIST ATTIVA: {len(self.whitelist_paths)} percorsi definiti.")
                      time.sleep(1)
                 else:
                      print(Fore.YELLOW + "Nessun percorso valido inserito. Si userà la ricerca standard (tutti i dischi).")
                      time.sleep(2)

        self.clear_screen()
        if not self.exclusions:
            if self.mode == "Android":
                self.exclusions = list(self.android_engine.system_folders)
            else:
                self.exclusions = list(self.engine.system_folders)

        print(Fore.CYAN + "Gestione Esclusioni")
        print(Fore.CYAN + "Definisci quali cartelle ignorare.")
        print(Fore.RED + "   - Cartelle nascoste (es. .git, .venv, $Recycle.Bin) sono SEMPRE escluse.")
        print(Fore.YELLOW + "\nEsclusioni attuali (Default):")
        for excl in self.exclusions:
            print(Fore.YELLOW + f"   - {excl}")
        print("")

        res = self._run_gum(["confirm", "--default=false", "Vuoi modificare la lista delle esclusioni?"])
        if res.returncode == 0:
            print(Fore.CYAN + "\nModifica i percorsi da escludere (uno per riga).")
            print(Fore.CYAN + "Premi CTRL+D (o CTRL+Z su Windows) poi INVIO per salvare.")
            current_value = "\n".join(self.exclusions)
            res_write = self._run_gum(["write", "--value", current_value, "--width", "80", "--height", "15"])
            if res_write.returncode == 0:
                paths = res_write.stdout.strip().split('\n')
                self.exclusions = []
                for p in paths:
                    clean_p = p.strip()
                    if clean_p:
                        self.exclusions.append(clean_p)
             
        if self.exclusions:
            print(Fore.GREEN + f"Lista esclusioni aggiornata: {len(self.exclusions)} regole attive.")
            time.sleep(1)

        self.exceptions = []
        # Pre-populate defaults if available (e.g. Android WhatsApp)
        if hasattr(self.engine, 'default_exceptions'):
             self.exceptions.extend(self.engine.default_exceptions)
        elif self.mode == "Android" and hasattr(self.android_engine, 'default_exceptions'):
             self.exceptions.extend(self.android_engine.default_exceptions)
             
        print(Fore.CYAN + "\nGestione Eccezioni (Inclusioni Forzate)")
        print(Fore.CYAN + "Definisci quali sottocartelle includere ANCHE SE la cartella padre è esclusa.")
        print(Fore.CYAN + "Esempio: Escludi 'Android' ma includi 'Android/media/com.whatsapp/Whatsapp'.")
        
        if self.exceptions:
             print(Fore.YELLOW + "Eccezioni predefinite attive:")
             for e in self.exceptions:
                 print(Fore.YELLOW + f"   - {e}")
        
        res = self._run_gum(["confirm", "--default=false", "Vuoi modificare le eccezioni?"])
        if res.returncode == 0:
             print(Fore.CYAN + "\nInserisci i percorsi da includere forzatamente (uno per riga).")
             print(Fore.CYAN + "Premi CTRL+D (o CTRL+Z su Windows) poi INVIO per terminare.")
             
             current_val = "\n".join(self.exceptions)
             placeholder = "Esempio:\n/sdcard/Android/media/com.whatsapp/Whatsapp" if self.mode == "Android" else "Esempio:\nC:\\Users\\Utente\\Documents\\CartellaSpeciale"
             
             res_write = self._run_gum(["write", "--value", current_val, "--placeholder", placeholder, "--width", "80", "--height", "10"])
             
             if res_write.returncode == 0:
                 paths = res_write.stdout.strip().split('\n')
                 self.exceptions = []
                 for p in paths:
                     clean_p = p.strip()
                     if clean_p:
                         self.exceptions.append(clean_p)
                         
        if self.exceptions:
            print(Fore.GREEN + f"Lista eccezioni aggiornata: {len(self.exceptions)} regole attive.")
            time.sleep(1)

    def step2_select_drive(self):
        self.print_header("Destinazione")
        while True:
            drives = self.engine.get_removable_drives()
            if not drives:
                print(Fore.RED + "Nessuna unità USB trovata!")
                
                # Fallback: ask for local folder
                res = self._run_gum(["confirm", "--default=false", "Nessuna USB trovata. Vuoi salvare i backup in una cartella specifica?"])
                if res.returncode == 0:
                    print(Fore.CYAN + "Inserisci il percorso completo della cartella di destinazione:")
                    res_input = self._run_gum(["input", "--placeholder", "Es: C:\\Backup o /Users/nome/Backup"])
                    custom_path = res_input.stdout.strip()
                    
                    if custom_path:
                        abs_path = os.path.abspath(custom_path)
                        
                        # Check/Create folder
                        if not os.path.exists(abs_path):
                            res_create = self._run_gum(["confirm", f"La cartella '{abs_path}' non esiste. Vuoi crearla?"])
                            if res_create.returncode == 0:
                                try:
                                    os.makedirs(abs_path)
                                except Exception as e:
                                    print(Fore.RED + f"Impossibile creare la cartella: {e}")
                                    time.sleep(2)
                                    continue
                            else:
                                continue
                        
                        if os.path.isdir(abs_path):
                            try:
                                usage = shutil.disk_usage(abs_path)
                                self.selected_drive = {
                                    'device': 'Local Folder',
                                    'mountpoint': abs_path,
                                    'free': usage.free,
                                    'total': usage.total
                                }
                                return # Drive selected, exit function
                            except Exception as e:
                                print(Fore.RED + f"Errore accesso cartella: {e}")
                                time.sleep(2)
                                continue
                        else:
                            print(Fore.RED + "Il percorso indicato non è una cartella.")
                            time.sleep(2)
                            continue
                
                res = self._run_gum(["confirm", "Riprovare?"])
                if res.returncode != 0: sys.exit()
                continue
            
            drive_map = {}
            choices = []
            for d in drives:
                label = f"{d['mountpoint']} [{d['device']}] ({d['free']//(1024**3)} GB Free)"
                drive_map[label] = d
                choices.append(label)
            
            choices.append("Aggiorna Lista")
            res = self._run_gum(["choose", "--header", "Scegli l'unità di destinazione"] + choices)
            selection = res.stdout.strip()
            if not selection: sys.exit()
            if selection == "Aggiorna Lista": continue
            self.selected_drive = drive_map[selection]
            break

    def step3_scan_and_confirm(self):
        self.print_header("Scansione")
        
        # Use the filtered map instead of raw categories
        cat_map = self.active_category_map
        
        all_scan_roots = []
        
        if self.mode == "PC":
            # 1. Identify all fixed drives
            fixed_drives = []
            if os.name == 'nt':
                for part in psutil.disk_partitions():
                    if 'fixed' in part.opts or 'rw,fixed' in part.opts:
                        fixed_drives.append(part.mountpoint)
            
            # Remove destination drive if it's in the list (to avoid recursion)
            dest_mount = self.selected_drive['mountpoint']
            if dest_mount in fixed_drives:
                fixed_drives.remove(dest_mount)
                
            # 2. Apply Whitelist Logic PER DRIVE
            # Helper to check if path is on drive
            def is_on_drive(path, drive_root):
                try:
                    # Normalize paths
                    p_abs = os.path.abspath(path).lower()
                    d_abs = os.path.abspath(drive_root).lower()
                    # Drive root usually ends with backslash (C:\), but if not we ensure it
                    if not d_abs.endswith(os.sep):
                        d_abs += os.sep
                    return p_abs.startswith(d_abs)
                except:
                    return False
                    
            used_whitelist_paths = []
            
            for drive in fixed_drives:
                # Find whitelist entries for this drive
                drive_specific_paths = []
                if self.whitelist_paths:
                    for wp in self.whitelist_paths:
                        if is_on_drive(wp, drive):
                            drive_specific_paths.append(wp)
                
                if drive_specific_paths:
                    # Whitelist applies for this drive: Scan ONLY specific paths
                    all_scan_roots.extend(drive_specific_paths)
                    used_whitelist_paths.extend(drive_specific_paths)
                else:
                    # No whitelist for this drive: Scan FULL drive
                    all_scan_roots.append(drive)
            
            # Also add any whitelist paths that might not match a fixed drive
            for wp in self.whitelist_paths:
                if wp not in used_whitelist_paths:
                    all_scan_roots.append(wp)
        else:
            # Android Mode
            all_scan_roots = ["Android Device"]

        # --- RESOCONTO PRE-SCANSIONE ---
        self.print_header("Resoconto Configurazioni")
        
        print(Fore.CYAN + "Stai per avviare la ricerca con queste impostazioni:\n")
        
        print(Fore.WHITE + Style.BRIGHT + "Modalità:")
        print(f"   - Tipo: {self.mode}")
        print(f"   - Test Mode: {'ATTIVA (Max 10 file)' if self.test_mode else 'Disattivata (Copia completa)'}")
        
        if self.mode == "PC":
            if self.whitelist_paths:
                 print(f"   - Strategia Scansione: MISTA (Whitelist per drive / Full per gli altri)")
            else:
                 print(f"   - Strategia Scansione: COMPLETA (Tutti i dischi fissi)")
        
        print(Fore.WHITE + Style.BRIGHT + "\nDestinazione:")
        print(f"   - Unità: {self.selected_drive['mountpoint']} [{self.selected_drive['device']}]")
        print(f"   - Spazio Libero: {self.selected_drive['free']//(1024**3)} GB")
        
        print(Fore.WHITE + Style.BRIGHT + "\nPercorsi di Scansione Effettivi:")
        if self.mode == "PC":
            for root in all_scan_roots:
                # Check if it's a full drive or a specific folder for display
                is_drive_root = False
                if os.name == 'nt':
                     if len(root) <= 3 and root.endswith(':\\'): is_drive_root = True
                
                if is_drive_root:
                    print(Fore.GREEN + f"   - {root} (Intero Disco)")
                else:
                    print(Fore.GREEN + f"   - {root} (Cartella Specifica)")
        else:
             print(Fore.GREEN + f"   - /sdcard (Memoria Interna Android)")

        if self.extra_folders: # Legacy check
             for f in self.extra_folders:
                 print(Fore.GREEN + f"   - {os.path.abspath(f)}")
            
        print(Fore.WHITE + Style.BRIGHT + "\nPercorsi ESCLUSI:")
        
        display_exclusions = self.exclusions
        
        system_excl_display = []
        custom_excl_display = []
        
        # Use correct system folders list for check
        sys_folders = self.android_engine.system_folders if self.mode == "Android" else self.engine.system_folders
        
        for excl in display_exclusions:
             if excl in sys_folders:
                 system_excl_display.append(excl)
             else:
                 custom_excl_display.append(excl)
                 
        if system_excl_display:
             print(Fore.RED + "   - [Sistema] Cartelle di sistema:")
             for f in system_excl_display:
                  print(Fore.RED + f"     * {f}")
                  
        if custom_excl_display:
            print(Fore.RED + "   - [Custom] Utente:")
            for e in custom_excl_display:
                 print(Fore.RED + f"     * {e}")
                 
        if self.mode == "PC":
            print(Fore.RED + "   - [Auto] Tutte le cartelle che iniziano con '.' (es. .git) o '$' (es. $Recycle.Bin)")
        else:
            print(Fore.RED + "   - [Auto] Cartelle nascoste (es. .thumbnails)")
            
        if self.exceptions:
            print(Fore.WHITE + Style.BRIGHT + "\nEccezioni (Inclusioni Forzate):")
            for exc in self.exceptions:
                print(Fore.GREEN + f"   - {exc}")

        print(Fore.WHITE + Style.BRIGHT + "\nFiltri Attivi:")
        for cat, exts in cat_map.items():
            print(f"   - {cat}: {', '.join(exts)}")
        
        if self.custom_extensions:
            print(f"   - Custom: {', '.join(self.custom_extensions)}")
            
        print("\n" + "-"*50 + "\n")
        
        # Gum confirm to proceed
        res = self._run_gum(["confirm", "Vuoi confermare le impostazioni e procedere con la scansione?"])
        if res.returncode != 0:
            print(Fore.RED + "Operazione annullata dall'utente.")
            sys.exit()

        print(Fore.YELLOW + "Avvio analisi file in corso...")
        
        if self.mode == "PC":
            files, size = self.engine.scan_files(all_scan_roots, cat_map, self.custom_extensions, self.exclusions, self.exceptions)
        else:
            files, size = self.android_engine.scan_files(self.android_device_id, cat_map, self.custom_extensions, self.exclusions, self.exceptions)
        
        if self.test_mode:
            print(Fore.YELLOW + "Applicazione filtro TEST MODE (max 10 file per categoria)...")
            filtered_files = []
            cat_counts = {}
            new_size = 0
            
            for f in files:
                cat = f['category']
                count = cat_counts.get(cat, 0)
                if count < 10:
                    filtered_files.append(f)
                    cat_counts[cat] = count + 1
                    new_size += f['size']
            
            files = filtered_files
            size = new_size

        self.print_header("Riepilogo")
        if self.test_mode:
            print(Fore.YELLOW + "[TEST MODE ATTIVA]")
            
        print(Fore.GREEN + f"Risultati Scansione:")
        print(f"  - File trovati: {len(files)}")
        print(f"  - Dimensione totale: {size / (1024*1024):.2f} MB")
        
        if not files:
            print(Fore.RED + "\nNessun file trovato.")
            sys.exit()

        res = self._run_gum(["confirm", "Vuoi procedere con il backup dei file trovati?"])
        if res.returncode != 0:
            sys.exit()
        
        return files, size

    def step4_perform_backup(self, files, total_size):
        self.print_header("Backup in corso")
        dest = self.selected_drive['mountpoint']
        
        pbar = tqdm(total=total_size, unit='B', unit_scale=True, desc="Copia", bar_format="{l_bar}{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}]")
        
        def progress_callback(action, filename, current_copied_size=0):
            delta = current_copied_size - pbar.n
            if delta > 0:
                pbar.update(delta)

        try:
            if self.mode == "PC":
                count, errors = self.engine.copy_files(files, dest, progress_callback=progress_callback)
            else:
                count, errors = self.android_engine.copy_files(files, dest, progress_callback=progress_callback)
                
            pbar.close()
            
            self.print_header("Completato")
            
            # Use gum style for success message
            self._run_gum(["style", "--foreground", "212", "--border", "rounded", "--align", "center", "--width", "50", f"Backup Completato!\n{count} file copiati."])
            
            if errors:
                print(Fore.RED + f"Si sono verificati {len(errors)} errori. Vedi 'backup_errors.log'")
                with open("backup_errors.log", "w") as f:
                    for e in errors: f.write(e + "\n")
            
            print("\nPremi INVIO per uscire...")
            input() # simple input to wait
                
        except KeyboardInterrupt:
            print(Fore.RED + "\n\nInterrotto!")
            if self.mode == "PC":
                self.engine.stop()
            else:
                self.android_engine.stop()

    def _check_and_install_adb(self):
        if os.name == 'nt':
            self._check_and_install_adb_windows()
        else:
            self._check_and_install_adb_macos()

    def _check_and_install_adb_windows(self):
        adb_found = shutil.which("adb") is not None
        if adb_found:
            return
        
        print(Fore.RED + "ADB non trovato! È necessario per il backup Android.")
        print(Fore.YELLOW + "\nNOTA: Se ADB è già stato installato in precedenza, potrebbe essere")
        print(Fore.YELLOW + "necessario un RIAVIO del sistema per rendere effettive le modifiche.\n")
        
        res = self._run_gum(["confirm", "--default=true", "Vuoi installare ADB automaticamente con winget?"])
        if res.returncode == 0:
            try:
                print(Fore.CYAN + "Installazione in corso...")
                result = subprocess.run(
                    ["winget", "install", "-e", "--id", "Google.PlatformTools"],
                    capture_output=True,
                    text=True
                )
                if result.returncode == 0:
                    print(Fore.GREEN + "\nADB installato con successo!")
                    print(Fore.YELLOW + "Riavvia l'app per continuare.")
                    sys.exit(0)
                else:
                    print(Fore.RED + f"Installazione fallita: {result.stderr}")
                    sys.exit(1)
            except Exception as e:
                print(Fore.RED + f"Errore durante l'installazione: {e}")
                sys.exit(1)
        else:
            print(Fore.RED + "ADB è necessario per il corretto funzionamento dell'app.")
            sys.exit(1)

    def _check_and_install_adb_macos(self):
        brew_found = shutil.which("brew") is not None
        
        if not brew_found:
            print(Fore.RED + "Homebrew non trovato! È necessario per installare ADB.")
            res = self._run_gum(["confirm", "--default=true", "Vuoi installare Homebrew automaticamente?"])
            if res.returncode == 0:
                try:
                    print(Fore.CYAN + "Installazione di Homebrew in corso...")
                    print(Fore.YELLOW + "Potrebbe essere richiesta la password dell'utente.\n")
                    result = subprocess.run(
                        ["/bin/bash", "-c", "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"],
                        capture_output=True,
                        text=True
                    )
                    if result.returncode == 0:
                        print(Fore.GREEN + "\nHomebrew installato con successo!")
                    else:
                        print(Fore.RED + f"Installazione Homebrew fallita: {result.stderr}")
                        sys.exit(1)
                except Exception as e:
                    print(Fore.RED + f"Errore durante l'installazione di Homebrew: {e}")
                    sys.exit(1)
            else:
                print(Fore.RED + "Homebrew è necessario per il corretto funzionamento dell'app.")
                sys.exit(1)
        
        adb_found = shutil.which("adb") is not None
        if adb_found:
            return
        
        print(Fore.RED + "ADB non trovato! È necessario per il backup Android.")
        print(Fore.YELLOW + "\nNOTA: Se ADB è già stato installato in precedenza, potrebbe essere")
        print(Fore.YELLOW + "necessario un RIAVIO del terminale per rendere effettive le modifiche.\n")
        
        res = self._run_gum(["confirm", "--default=true", "Vuoi installare ADB automaticamente con Homebrew?"])
        if res.returncode == 0:
            try:
                print(Fore.CYAN + "Installazione di ADB in corso...")
                result = subprocess.run(
                    ["brew", "install", "android-platform-tools"],
                    capture_output=True,
                    text=True
                )
                if result.returncode == 0:
                    print(Fore.GREEN + "\nADB installato con successo!")
                    print(Fore.YELLOW + "Riavvia l'app per continuare.")
                    sys.exit(0)
                else:
                    print(Fore.RED + f"Installazione ADB fallita: {result.stderr}")
                    sys.exit(1)
            except Exception as e:
                print(Fore.RED + f"Errore durante l'installazione di ADB: {e}")
                sys.exit(1)
        else:
            print(Fore.RED + "ADB è necessario per il corretto funzionamento dell'app.")
            sys.exit(1)

    def run(self):
        self._check_and_install_adb()
        try:
            self.step_select_mode()
            self.step1_select_filters()
            self.step2_select_drive()
            files, size = self.step3_scan_and_confirm()
            self.step4_perform_backup(files, size)
        except KeyboardInterrupt:
            print("\nUscita.")

if __name__ == "__main__":
    app = GumBackupApp()
    app.run()
