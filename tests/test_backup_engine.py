import os
import sys
import pytest
import threading
from unittest.mock import patch, MagicMock, mock_open

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from backup_engine import BackupEngine


class TestBackupEngineInit:
    def test_categories_defined(self):
        engine = BackupEngine()
        assert "Documenti" in engine.categories
        assert "Immagini" in engine.categories
        assert "Video" in engine.categories
        assert "Audio" in engine.categories
        assert "Archivi" in engine.categories

    def test_categories_have_extensions(self):
        engine = BackupEngine()
        for category, extensions in engine.categories.items():
            assert isinstance(extensions, list)
            assert all(ext.startswith('.') for ext in extensions)

    def test_system_folders_darwin(self, mock_platform):
        with mock_platform('Darwin'):
            engine = BackupEngine()
            assert "System" in engine.system_folders
            assert "Library" in engine.system_folders

    def test_system_folders_windows(self, mock_platform):
        with mock_platform('Windows'):
            engine = BackupEngine()
            assert "Windows" in engine.system_folders
            assert "Program Files" in engine.system_folders

    def test_default_events(self):
        engine = BackupEngine()
        assert engine.stop_event is not None
        assert engine.pause_event is not None
        assert engine.pause_event.is_set()


class TestGetRemovableDrives:
    def test_get_removable_drives_darwin_no_volumes(self, mock_platform, mock_psutil):
        with mock_platform('Darwin'), mock_psutil:
            engine = BackupEngine()
            drives = engine.get_removable_drives()
            assert isinstance(drives, list)

    def test_get_removable_drives_windows_no_removable(self, mock_platform, mock_psutil):
        with mock_platform('Windows'), mock_psutil:
            engine = BackupEngine()
            drives = engine.get_removable_drives()
            assert isinstance(drives, list)


class TestScanFiles:
    def test_scan_files_returns_empty_when_no_sources(self):
        engine = BackupEngine()
        files, size = engine.scan_files([], {}, [], [])
        assert files == []
        assert size == 0

    def test_scan_files_with_nonexistent_source(self):
        engine = BackupEngine()
        files, size = engine.scan_files(["/nonexistent/path"], {"Documenti": [".pdf"]}, [], [])
        assert files == []
        assert size == 0

    def test_scan_files_filters_by_extension(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        test_file = temp_dir / "test.pdf"
        test_file.write_text("test content")
        
        engine = BackupEngine()
        files, size = engine.scan_files([str(temp_dir)], {"Documenti": [".pdf"]}, [], [])
        
        assert len(files) == 1
        assert files[0]['source'].endswith("test.pdf")
        assert files[0]['category'] == "Documenti"

    def test_scan_files_excludes_hidden_folders(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        hidden_dir = temp_dir / ".hidden"
        hidden_dir.mkdir()
        (hidden_dir / "secret.pdf").write_text("secret")
        
        visible_file = temp_dir / "visible.pdf"
        visible_file.write_text("visible")
        
        engine = BackupEngine()
        files, size = engine.scan_files([str(temp_dir)], {"Documenti": [".pdf"]}, [], [])
        
        sources = [f['source'] for f in files]
        assert any("visible.pdf" in s for s in sources)
        assert not any("secret.pdf" in s for s in sources)

    def test_scan_files_includes_exceptions(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        android_dir = temp_dir / "Android"
        android_dir.mkdir()
        
        whatsapp_dir = android_dir / "media" / "com.whatsapp"
        whatsapp_dir.mkdir(parents=True)
        (whatsapp_dir / "important.txt").write_text("important")
        
        engine = BackupEngine()
        
        exclusions = ["Android"]
        exceptions = [str(whatsapp_dir)]
        
        files, size = engine.scan_files(
            [str(temp_dir)],
            {"Documenti": [".txt"]},
            [],
            exclusions,
            exceptions
        )
        
        sources = [f['source'] for f in files]
        assert any("important.txt" in s for s in sources)

    def test_scan_files_custom_extensions(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        (temp_dir / "file.log").write_text("log")
        
        engine = BackupEngine()
        files, size = engine.scan_files(
            [str(temp_dir)],
            {},
            [".log"],
            []
        )
        
        assert len(files) == 1
        assert files[0]['source'].endswith("file.log")


class TestCopyFiles:
    def test_copy_files_creates_destination(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        source_file = temp_dir / "source.txt"
        source_file.write_text("content")
        
        dest_dir = temp_dir / "backup"
        dest_dir.mkdir()
        
        engine = BackupEngine()
        files = [{
            'source': str(source_file),
            'rel_path': 'source.txt',
            'size': 7,
            'category': 'Documenti',
            'root_label': ''
        }]
        
        count, errors = engine.copy_files(files, str(dest_dir))
        
        assert count == 1
        assert len(errors) == 0
        assert any(p.name == "source.txt" for p in dest_dir.rglob("source.txt"))

    def test_copy_files_verifies_md5(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        source_file = temp_dir / "verify.txt"
        content = "content to verify"
        source_file.write_text(content)
        
        dest_dir = temp_dir / "backup"
        dest_dir.mkdir()
        
        engine = BackupEngine()
        files = [{
            'source': str(source_file),
            'rel_path': 'verify.txt',
            'size': len(content),
            'category': 'Documenti',
            'root_label': ''
        }]
        
        count, errors = engine.copy_files(files, str(dest_dir), verify=True)
        
        assert count == 1

    def test_copy_files_respects_stop_event(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        for i in range(5):
            (temp_dir / f"file{i}.txt").write_text(f"content{i}")
        
        dest_dir = temp_dir / "backup"
        dest_dir.mkdir()
        
        engine = BackupEngine()
        engine.stop_event.set()
        
        files = [{
            'source': str(temp_dir / f"file{i}.txt"),
            'rel_path': f"file{i}.txt",
            'size': 10,
            'category': 'Documenti',
            'root_label': ''
        } for i in range(5)]
        
        count, errors = engine.copy_files(files, str(dest_dir))
        
        assert count < 5

    def test_copy_files_respects_pause_event(self, temp_dir, mock_platform):
        mock_platform('Darwin')
        
        source_file = temp_dir / "paused.txt"
        source_file.write_text("content")
        
        dest_dir = temp_dir / "backup"
        dest_dir.mkdir()
        
        engine = BackupEngine()
        engine.pause_event.clear()
        
        files = [{
            'source': str(source_file),
            'rel_path': "paused.txt",
            'size': 7,
            'category': 'Documenti',
            'root_label': ''
        }]

        result = {}

        def run_copy():
            result["count"], result["errors"] = engine.copy_files(files, str(dest_dir))

        worker = threading.Thread(target=run_copy, daemon=True)
        worker.start()
        worker.join(timeout=0.2)

        # While paused, copy loop should still be blocked.
        assert worker.is_alive()

        engine.resume()
        worker.join(timeout=1.0)

        assert not worker.is_alive()
        assert result["count"] == 1
        assert result["errors"] == []


class TestStopPause:
    def test_stop(self):
        engine = BackupEngine()
        engine.stop()
        assert engine.stop_event.is_set()

    def test_pause(self):
        engine = BackupEngine()
        engine.pause()
        assert not engine.pause_event.is_set()

    def test_resume(self):
        engine = BackupEngine()
        engine.pause()
        engine.resume()
        assert engine.pause_event.is_set()