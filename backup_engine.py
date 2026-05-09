import os
import shutil
import psutil
import hashlib
import threading
import time
import platform
import subprocess
from pathlib import Path

# Read chunk size for MD5 verification (larger buffer reduces syscall overhead).
HASH_CHUNK_SIZE = 1024 * 1024


class BackupEngine:
    def __init__(self):
        self.stop_event = threading.Event()
        self.pause_event = threading.Event()
        self.pause_event.set()  # Start unpaused
        self.current_status = "Ready"
        
        # Default Categories
        self.categories = {
            "Documenti": [".doc", ".docx", ".pdf", ".txt", ".xlsx", ".xls", ".pptx", ".ppt", ".odt"],
            "Immagini": [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".raw", ".ico", ".heic", ".heif", ".webp"],
            "Video": [".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm"],
            "Audio": [".mp3", ".wav", ".aac", ".flac", ".ogg", ".wma", ".m4a"],
            "Archivi": [".zip", ".rar", ".7z", ".tar", ".gz"]
        }
        
        if platform.system() == 'Darwin':
            self.system_folders = [
                "System", "Library", "Applications", "private", "usr", "var", 
                ".Spotlight-V100", ".fseventsd", "Volumes"
            ]
        else:
            self.system_folders = [
                "Windows", "Program Files", "Program Files (x86)", "ProgramData", 
                "$Recycle.Bin", "System Volume Information", "AppData",
                "Boot", "Recovery", "PerfLogs", "Config.Msi", "Documents and Settings",
                "MSOCache", "$WinREAgent", "OneDriveTemp", "Temp", "XboxGames", "WindowsApps", "Windows.old"
            ]

    def get_removable_drives(self):
        drives = []
        if platform.system() == 'Darwin':
            seen = set()
            for part in psutil.disk_partitions():
                if part.mountpoint.startswith('/Volumes/'):
                    try:
                        usage = psutil.disk_usage(part.mountpoint)
                        key = part.mountpoint
                        if key in seen:
                            continue
                        seen.add(key)
                        drives.append({
                            'device': part.device,
                            'mountpoint': part.mountpoint,
                            'fstype': part.fstype,
                            'total': usage.total,
                            'free': usage.free
                        })
                    except Exception:
                        pass
            if not drives and os.path.isdir('/Volumes'):
                for name in os.listdir('/Volumes'):
                    mp = os.path.join('/Volumes', name)
                    if os.path.ismount(mp):
                        try:
                            usage = psutil.disk_usage(mp)
                            drives.append({
                                'device': mp,
                                'mountpoint': mp,
                                'fstype': '',
                                'total': usage.total,
                                'free': usage.free
                            })
                        except Exception:
                            pass
        else:
            for part in psutil.disk_partitions():
                if 'removable' in part.opts or part.opts == 'rw,removable':
                    try:
                        usage = psutil.disk_usage(part.mountpoint)
                        drives.append({
                            'device': part.device,
                            'mountpoint': part.mountpoint,
                            'fstype': part.fstype,
                            'total': usage.total,
                            'free': usage.free
                        })
                    except Exception:
                        pass
        return drives

    def scan_files(self, source_drives, category_extensions_map, custom_extensions, exclusions, exceptions=None, progress_callback=None):
        """
        Scans drives for files matching criteria.
        category_extensions_map: dict { "CategoryName": [".ext1", ".ext2"] }
        """
        if exceptions is None:
            exceptions = []
            
        files_to_copy = []
        total_size = 0
        
        # Compile allowed extensions and mapping
        allowed_exts = set()
        ext_to_cat = {}
        
        for cat, exts in category_extensions_map.items():
            for e in exts:
                allowed_exts.add(e)
                ext_to_cat[e] = cat
                
        for e in custom_extensions:
            allowed_exts.add(e)
            if e not in ext_to_cat:
                ext_to_cat[e] = "Altro"
                
        allowed_exts = {e.lower() for e in allowed_exts}

        for drive in source_drives:
            drive_path = Path(drive)
            root_label = ""
            if platform.system() == 'Darwin':
                p = drive_path.as_posix()
                if p.startswith('/Volumes/'):
                    parts = drive_path.parts
                    root_label = parts[2] if len(parts) > 2 else "Volume"
                else:
                    home = Path(os.path.expanduser("~"))
                    try:
                        if drive_path == home or str(drive_path).startswith(str(home)):
                            root_label = "Home"
                    except Exception:
                        root_label = ""
            else:
                root_label = drive_path.anchor.replace(":", "")
            
            # Special handling for System Drive (usually C:)
            # If scanning C: ROOT, strictly limit to C:\Users
            # If scanning a specific folder on C: (e.g. C:\Work), scan that folder.
            limit_to_user_profile = False
            
            if os.name == 'nt' and os.getenv('SystemDrive'):
                system_drive = os.getenv('SystemDrive').upper() # e.g. "C:"
                current_drive_root = drive_path.anchor.rstrip('\\/') # e.g. "C:"
                
                # Check if the path matches the system drive
                if current_drive_root.upper() == system_drive:
                    # Check if we are scanning the ROOT of the drive
                    # Comparison should be case-insensitive and ignore trailing slashes
                    abs_input = os.path.abspath(drive).rstrip('\\/').upper()
                    abs_root = os.path.abspath(drive_path.anchor).rstrip('\\/').upper()
                    
                    if abs_input == abs_root:
                        limit_to_user_profile = True
                    
            start_paths = [drive_path]
            if limit_to_user_profile:
                # On system drive ROOT, only scan CURRENT User folder
                # e.g. C:\Users\NomeUtente
                try:
                    user_path = Path(os.path.expanduser("~"))
                    # Verify this user path is actually on the drive we are scanning
                    if user_path.anchor.upper().rstrip('\\/') == current_drive_root.upper():
                        start_paths = [user_path]
                    else:
                        # Fallback: if user profile is not on system drive (rare setup), scan Users root
                        users_path = drive_path / "Users"
                        start_paths = [users_path] if users_path.exists() else [drive_path]
                except Exception:
                    # Fallback on error
                    users_path = drive_path / "Users"
                    start_paths = [users_path] if users_path.exists() else [drive_path]
            
            for start_path in start_paths:
                for root, dirs, files in os.walk(start_path):
                    # Check Stop/Pause
                    if self.stop_event.is_set():
                        return [], 0
                    
                    # 1. Prune subdirectories
                    dirs[:] = [d for d in dirs if not self._is_excluded(Path(root) / d, exclusions, exceptions)]
                    
                    # 2. Check if CURRENT root is excluded
                    if self._is_excluded(Path(root), exclusions, exceptions):
                        continue

                    for file in files:
                        try:
                            file_path = Path(root) / file
                            ext = file_path.suffix.lower()
                            
                            if ext in allowed_exts:
                                # Check size (might raise PermissionError)
                                stat = file_path.stat()
                                size = stat.st_size
                                category = ext_to_cat.get(ext, "Altro")
                                
                                files_to_copy.append({
                                    'source': str(file_path),
                                    'size': size,
                                    'category': category,
                                    'rel_path': str(file_path.relative_to(drive_path)),
                                    'root_label': root_label
                                })
                                total_size += size
                                
                                if progress_callback:
                                    progress_callback("scanning", file_path.name)
                        except Exception:
                            continue

        return files_to_copy, total_size

    def _is_excluded(self, path, exclusions, exceptions=None):
        """Check if path matches any exclusion criteria."""
        path = Path(path)
        if exceptions is None:
            exceptions = []
        
        # Check hidden attribute (Windows) or dotfile (Unix/Windows)
        # Note: On Windows, checking FILE_ATTRIBUTE_HIDDEN is more robust, but stat().st_file_attributes is platform dependent.
        # For cross-platform simplicity, we check if any part starts with '.' (common convention)
        # AND on Windows specifically we can try to check the attribute if needed, but 'os.walk' doesn't give us attributes directly.
        # Let's implement a basic check: skip if name starts with '.' OR if we can detect hidden attribute.
        
        # 1. Check if any part of the path starts with '.' (covers .git, .venv, etc.) or '$' (system/hidden/temp)
        for part in path.parts:
            if part.startswith('.') or part.startswith('$'):
                return True

        # 2. Check Windows hidden attribute for the path itself (if it exists)
        if os.name == 'nt':
            try:
                import stat
                if path.exists():
                    attrs = path.stat().st_file_attributes
                    if attrs & stat.FILE_ATTRIBUTE_HIDDEN:
                        return True
            except Exception:
                pass

        # Check system folder names (exact match on parts)
        # Note: This is now handled via passed 'exclusions' list for flexibility, 
        # but we keep this check if user didn't override the list to be safe.
        # However, to allow user to Remove system folders from exclusion, we rely on the passed list.
        # If the passed list is empty, we might assume no exclusions, but usually it will contain at least defaults.
        # To be safe, we check if the exclusion list matches one of the system folders.
        
        # Check custom exclusions (path starts with exclusion)
        path_str = str(path).lower()
        is_excluded = False
        
        for excl in exclusions:
            excl_str = str(excl).lower()
            # Check if exclusion is a direct parent or match
            # We use startswith on string representation for simple tree exclusion
            if path_str.startswith(excl_str):
                is_excluded = True
                break
                
            # Also check if a specific part matches the exclusion (for folder names like 'Windows' appearing anywhere)
            if os.sep not in excl_str and '/' not in excl_str:
                 for part in path.parts:
                     if part.lower() == excl_str:
                         is_excluded = True
                         break
            
            if is_excluded:
                break

        if not is_excluded:
            return False

        # If excluded, check if it is saved by an exception (inclusion)
        # 1. Is path a parent of an exception? (We must traverse it to find the exception)
        # 2. Is path the exception itself or inside it?
        
        for exc in exceptions:
            exc_path = Path(exc)
            
            # Case 1: Path is parent of Exception
            # e.g. Path=/A, Exc=/A/B
            try:
                exc_path.relative_to(path)
                return False # Don't exclude, we need to go deeper
            except ValueError:
                pass
            
            # Case 2: Path is inside Exception (or is Exception)
            # e.g. Path=/A/B/C, Exc=/A/B
            try:
                path.relative_to(exc_path)
                return False # It is explicitly included
            except ValueError:
                pass
                
        return True

    def _get_category(self, ext):
        for cat, exts in self.categories.items():
            if ext in exts:
                return cat
        return "Altro"

    def copy_files(self, files_list, destination_root, verify=True, progress_callback=None):
        """
        Copies files to destination.
        Structure: Destination / Category / DriveLetter_Structure / ...
        """
        copied_count = 0
        copied_size = 0
        errors = []

        dest_path = Path(destination_root) / f"Backup_{int(time.time())}"
        dest_path.mkdir(parents=True, exist_ok=True)
        created_dirs = {dest_path}

        for file_info in files_list:
            # Check Pause
            self.pause_event.wait()
            
            # Check Stop
            if self.stop_event.is_set():
                break

            src = Path(file_info['source'])
            label = file_info.get('root_label', "")
            target_structure = Path(file_info['rel_path'])
            dst = dest_path / file_info['category'] / label / target_structure
            
            try:
                parent = dst.parent
                if parent not in created_dirs:
                    parent.mkdir(parents=True, exist_ok=True)
                    created_dirs.add(parent)
                
                if progress_callback:
                    progress_callback("copying", src.name, copied_size)

                shutil.copy2(src, dst)
                
                if verify:
                    if not self._verify_file(src, dst):
                        errors.append(f"Integrity check failed: {src}")
                
                copied_count += 1
                copied_size += file_info['size']
                
            except Exception as e:
                errors.append(f"Error copying {src}: {str(e)}")

        return copied_count, errors

    def _verify_file(self, src, dst):
        """Simple size check + optional hash check for critical verification."""
        # Fast check: Size
        if src.stat().st_size != dst.stat().st_size:
            return False
        # Deep check: MD5 (optional, can be slow for large files)
        # For this tool, we'll do a partial hash or full hash depending on requirements.
        # Let's do a full hash for safety as requested.
        return self._get_hash(src) == self._get_hash(dst)

    def _get_hash(self, path):
        hash_md5 = hashlib.md5()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(HASH_CHUNK_SIZE), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()

    def stop(self):
        self.stop_event.set()
        self.pause_event.set() # Ensure we don't get stuck in pause

    def pause(self):
        self.pause_event.clear()

    def resume(self):
        self.pause_event.set()


class AndroidBackupEngine(BackupEngine):
    def __init__(self):
        super().__init__()
        self.adb_exe = self._find_adb()
        self.system_folders = [
            "/Android/data", "/Android/obb", "/.thumbnails", "/.cache", "/LOST.DIR"
        ]
        self.default_exceptions = [
            "/sdcard/Android/media/com.whatsapp/WhatsApp/Media"
        ]

    def _find_adb(self):
        # Check if adb is in path
        if shutil.which("adb"):
            return "adb"
        # Check current directory
        if os.path.exists(os.path.join(os.getcwd(), "adb.exe")):
            return os.path.join(os.getcwd(), "adb.exe")
        if os.path.exists(os.path.join(os.getcwd(), "adb")):
            return os.path.join(os.getcwd(), "adb")
        return None

    def get_devices(self):
        if not self.adb_exe:
            return []
        try:
            output = subprocess.check_output([self.adb_exe, "devices", "-l"], text=True)
            devices = []
            for line in output.splitlines()[1:]: # Skip header
                if not line.strip(): continue
                parts = line.split()
                if len(parts) >= 2 and parts[1] != "offline" and parts[1] != "unauthorized":
                     # Format: serial device product:X model:Y device:Z
                     model = "Unknown"
                     for p in parts:
                         if p.startswith("model:"):
                             model = p.split(":")[1]
                     devices.append({
                         'id': parts[0],
                         'model': model,
                         'status': parts[1]
                     })
            return devices
        except Exception:
            return []

    def scan_files(self, device_id, category_extensions_map, custom_extensions, exclusions, exceptions=None, progress_callback=None):
        if not self.adb_exe:
            return [], 0
            
        if exceptions is None:
            exceptions = []

        files_to_copy = []
        total_size = 0
        
        # Compile allowed extensions
        allowed_exts = set()
        ext_to_cat = {}
        for cat, exts in category_extensions_map.items():
            for e in exts:
                allowed_exts.add(e)
                ext_to_cat[e] = cat
        for e in custom_extensions:
            allowed_exts.add(e)
            if e not in ext_to_cat:
                ext_to_cat[e] = "Altro"
        allowed_exts = {e.lower() for e in allowed_exts}

        # Scan /sdcard: list top-level entries, then per-folder find + stat (fallback: find paths only).
        base_path = "/sdcard"
        folders_to_scan = []
        try:
            ls_cmd = [self.adb_exe, "-s", device_id, "shell", "ls", "-1", base_path]
            ls_out = subprocess.check_output(ls_cmd, text=True, encoding='utf-8', errors='ignore')
            for line in ls_out.splitlines():
                folder = line.strip()
                if not folder: continue
                # Skip known bad/system folders
                if folder in ["lost+found", ".thumbnails", ".cache", "Backups", ".git"]:
                    continue
                if folder.startswith("."):
                    continue
                
                folders_to_scan.append(f"{base_path}/{folder}")
        except Exception:
            # Fallback if ls fails: use standard folders
            folders_to_scan = [
                f"{base_path}/DCIM", f"{base_path}/Pictures", f"{base_path}/Download", 
                f"{base_path}/Documents", f"{base_path}/Music", f"{base_path}/Movies",
                f"{base_path}/WhatsApp", f"{base_path}/Telegram"
            ]

        # 2. Scan each folder individually
        for folder in folders_to_scan:
            # Try with stat first for this folder
            try:
                # Construct stat command for this specific folder
                # We still prune thumbnails/cache if found inside
                stat_cmd = f"find '{folder}' \\( -path '*/.thumbnails' -o -path '*/.cache' -o -path '*/Android/data' \\) -prune -o -type f -exec stat -c '%s|%n' {{}} +"
                
                result = subprocess.run(
                    [self.adb_exe, "-s", device_id, "shell", stat_cmd],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    encoding='utf-8',
                    errors='ignore',
                    check=False
                )
                
                output = result.stdout
                if not output and result.returncode != 0:
                     # If stat failed, try simple find for this folder immediately
                     raise Exception("Stat failed")

                for line in output.splitlines():
                    if "|" not in line: continue
                    try:
                        size_str, path = line.split("|", 1)
                        size = int(size_str)
                        path = path.strip()
                        
                        # Check for duplicates (files_to_copy is list of dicts, inefficient check but safe for small counts)
                        # For speed, we trust path uniqueness from find
                        
                        ext = os.path.splitext(path)[1].lower()
                        if ext in allowed_exts:
                            if self._is_excluded_android(path, exclusions, exceptions): continue
                            cat = ext_to_cat.get(ext, "Altro")
                            
                            # Simplify path if it matches an exception
                            rel_path = self._simplify_exception_path(path, base_path, exceptions)
                            
                            files_to_copy.append({
                                'source': path,
                                'size': size,
                                'category': cat,
                                'rel_path': rel_path,
                                'root_label': "Android",
                                'device_id': device_id
                            })
                            total_size += size
                            if progress_callback: progress_callback("scanning", os.path.basename(path))
                    except ValueError:
                        continue
                        
            except Exception:
                # Fallback to simple find for this folder
                try:
                    find_cmd = f"find '{folder}' \\( -path '*/.thumbnails' -o -path '*/.cache' -o -path '*/Android/data' \\) -prune -o -type f -print"
                    result = subprocess.run(
                        [self.adb_exe, "-s", device_id, "shell", find_cmd],
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                        encoding='utf-8',
                        errors='ignore',
                        check=False
                    )
                    for line in result.stdout.splitlines():
                        path = line.strip()
                        if not path: continue
                        
                        ext = os.path.splitext(path)[1].lower()
                        if ext in allowed_exts:
                            if self._is_excluded_android(path, exclusions, exceptions): continue
                            cat = ext_to_cat.get(ext, "Altro")
                            
                            # Simplify path if it matches an exception
                            rel_path = self._simplify_exception_path(path, base_path, exceptions)
                            
                            files_to_copy.append({
                                'source': path,
                                'size': 0,
                                'category': cat,
                                'rel_path': rel_path,
                                'root_label': "Android",
                                'device_id': device_id
                            })
                            if progress_callback: progress_callback("scanning", os.path.basename(path))
                except Exception:
                    pass

        return files_to_copy, total_size

    def _simplify_exception_path(self, path, base_path, exceptions):
        """
        If path is inside an exception, we want to flatten it.
        E.g. Exception: /sdcard/Android/media/com.whatsapp/WhatsApp/Media
             Path: /sdcard/Android/media/com.whatsapp/WhatsApp/Media/WhatsApp Images/Img.jpg
             Result: WhatsApp Images/Img.jpg
             
        If not inside exception, return standard relative path.
        """
        if not exceptions:
            return os.path.relpath(path, base_path)
            
        p = path.replace("\\", "/").lower()
        
        best_exc = None
        best_len = 0
        
        for exc in exceptions:
            e = exc.replace("\\", "/").lower()
            if p.startswith(e):
                if len(e) > best_len:
                    best_len = len(e)
                    best_exc = exc
        
        if best_exc:
            # Found an exception container
            # We want the relative path from the exception root
            # BUT: If the exception points to .../WhatsApp/Media, 
            # and file is .../WhatsApp/Media/Images/img.jpg
            # we want Images/img.jpg.
            
            # relpath needs correct case if possible, but we only have 'path' string.
            # path is the full remote path.
            # best_exc is the user provided exception string.
            
            # Let's try to do string manipulation carefully
            # Remove the exception prefix from path
            # path: /sdcard/.../Media/Folder/File
            # exc:  /sdcard/.../Media
            
            # Case insensitive match for prefix length
            prefix_len = len(best_exc)
            
            # Check if separator is needed
            rel = path[prefix_len:]
            if rel.startswith("/"):
                rel = rel[1:]
                
            return rel
            
        return os.path.relpath(path, base_path)

    def _is_excluded_android(self, path, exclusions, exceptions=None):
        # Path is a string here (remote path)
        # exclusions are list of strings
        if exceptions is None:
            exceptions = []
            
        p = path.replace("\\", "/").lower()
        is_excluded = False
        
        for excl in exclusions:
            # clean path separators
            e = excl.replace("\\", "/").lower()
            if e in p: # Simple substring match for now
                 is_excluded = True
                 break
        
        if not is_excluded:
            return False
            
        # If excluded, check exceptions
        for exc in exceptions:
            exc = exc.replace("\\", "/").lower()
            # 1. Is path parent of exception? (Shouldn't happen here as we are checking files, but maybe folders?)
            # 2. Is path inside exception?
            if p.startswith(exc):
                return False
                
        return True

    def copy_files(self, files_list, destination_root, verify=True, progress_callback=None):
        copied_count = 0
        copied_size = 0
        errors = []
        
        if not self.adb_exe:
            return 0, ["ADB not found"]

        dest_path = Path(destination_root) / f"Android_Backup_{int(time.time())}"
        dest_path.mkdir(parents=True, exist_ok=True)
        
        for file_info in files_list:
             # Check Stop
            if self.stop_event.is_set():
                break
                
            src = file_info['source']
            device_id = file_info.get('device_id')
            rel = file_info['rel_path']
            cat = file_info['category']
            
            # On Android, user requested to keep original structure instead of categories
            # rel is already relative to /sdcard (e.g. DCIM/Camera/img.jpg)
            dst = dest_path / rel
            
            try:
                dst.parent.mkdir(parents=True, exist_ok=True)
                
                if progress_callback:
                    progress_callback("copying", os.path.basename(src), copied_size)
                
                # adb pull
                cmd = [self.adb_exe, "-s", device_id, "pull", src, str(dst)]
                subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                
                copied_count += 1
                copied_size += file_info['size']
                
            except Exception as e:
                errors.append(f"Error pulling {src}: {e}")
                
        return copied_count, errors
