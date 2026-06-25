import os
import sys
import pytest
from unittest.mock import patch, MagicMock

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from backup_engine import AndroidBackupEngine


class TestAndroidBackupEngineInit:
    def test_inherits_from_backup_engine(self):
        engine = AndroidBackupEngine()
        from backup_engine import BackupEngine
        assert isinstance(engine, BackupEngine)

    def test_system_folders_android(self):
        engine = AndroidBackupEngine()
        assert "/Android/data" in engine.system_folders
        assert "/Android/obb" in engine.system_folders
        assert "/.thumbnails" in engine.system_folders
        assert "/.cache" in engine.system_folders

    def test_default_exceptions(self):
        engine = AndroidBackupEngine()
        assert len(engine.default_exceptions) > 0
        assert any("whatsapp" in exc.lower() for exc in engine.default_exceptions)


class TestFindAdb:
    def test_adb_in_path(self, mock_shutil_which):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            assert engine.adb_exe == "adb"

    def test_adb_not_in_path(self, mock_shutil_which):
        with mock_shutil_which(None):
            engine = AndroidBackupEngine()
            assert engine.adb_exe is None

    def test_adb_local_exe_windows(self, mock_os_name, mock_shutil_which):
        with mock_os_name('nt'), mock_shutil_which(None):
            with patch('os.path.exists', return_value=True):
                engine = AndroidBackupEngine()
                assert engine.adb_exe is not None

    def test_adb_not_found_anywhere(self, mock_os_name, mock_shutil_which, mock_platform):
        with mock_os_name('posix'), mock_shutil_which(None), mock_platform('Darwin'):
            with patch('os.path.exists', return_value=False):
                engine = AndroidBackupEngine()
                assert engine.adb_exe is None


class TestGetDevices:
    def test_get_devices_returns_empty_when_no_adb(self):
        engine = AndroidBackupEngine()
        engine.adb_exe = None
        devices = engine.get_devices()
        assert devices == []

    def test_get_devices_parses_output(self, mock_shutil_which):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            engine.adb_exe = "adb"
            
            with patch('subprocess.check_output') as mock_output:
                mock_output.return_value = """List of devices attached
device1 device model:Pixel_7
device2 offline
"""
                devices = engine.get_devices()
                
                assert len(devices) == 1
                assert devices[0]['id'] == 'device1'
                assert devices[0]['model'] == 'Pixel_7'
                assert devices[0]['status'] == 'device'

    def test_get_devices_skips_offline(self, mock_shutil_which):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            engine.adb_exe = "adb"
            
            with patch('subprocess.check_output') as mock_output:
                mock_output.return_value = """List of devices attached
device1 device
device2 offline
device3 unauthorized
"""
                devices = engine.get_devices()
                
                assert len(devices) == 1
                assert devices[0]['id'] == 'device1'

    def test_get_devices_handles_exception(self, mock_shutil_which):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            engine.adb_exe = "adb"
            
            with patch('subprocess.check_output', side_effect=Exception("ADB error")):
                devices = engine.get_devices()
                
                assert devices == []


class TestAndroidScanFiles:
    def test_scan_files_no_device(self):
        engine = AndroidBackupEngine()
        engine.adb_exe = None
        files, size = engine.scan_files("device123", {}, [], [])
        assert files == []
        assert size == 0

    def test_scan_files_returns_empty_on_error(self, mock_shutil_which):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            engine.adb_exe = "adb"
            
            with patch('subprocess.run', side_effect=Exception("ADB error")):
                files, size = engine.scan_files("device123", {"Documenti": [".pdf"]}, [], [])
                assert files == []
                assert size == 0


class TestAndroidCopyFiles:
    def test_copy_files_no_adb(self):
        engine = AndroidBackupEngine()
        engine.adb_exe = None
        count, errors = engine.copy_files([], "/destination")
        assert count == 0
        assert len(errors) > 0

    def test_copy_files_uses_adb_pull(self, mock_shutil_which, tmp_path):
        with mock_shutil_which("adb"):
            engine = AndroidBackupEngine()
            engine.adb_exe = "adb"
            
            dest = tmp_path / "backup"
            dest.mkdir()
            
            files = [{
                'source': '/sdcard/test.pdf',
                'rel_path': 'test.pdf',
                'device_id': 'device123',
                'size': 1000,
                'category': 'Documenti'
            }]
            
            with patch('subprocess.run') as mock_run:
                mock_run.return_value = MagicMock(returncode=0)
                
                count, errors = engine.copy_files(files, str(dest))
                
                assert mock_run.called
                call_args = mock_run.call_args[0][0]
                assert 'pull' in call_args
                assert 'test.pdf' in str(call_args)