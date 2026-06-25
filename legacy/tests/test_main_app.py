import os
import sys
import pytest
from unittest.mock import patch, MagicMock, PropertyMock

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from main import GumBackupApp


class TestGumBackupAppInit:
    def test_default_values(self):
        app = GumBackupApp()
        
        assert app.mode == "PC"
        assert app.selected_categories == []
        assert app.custom_extensions == []
        assert app.exclusions == []
        assert app.exceptions == []
        assert app.selected_drive is None
        assert app.test_mode is False

    def test_whitelist_default(self):
        app = GumBackupApp()
        
        assert len(app.whitelist_paths) > 0
        assert os.path.expanduser("~") in app.whitelist_paths

    def test_gum_exe_detection(self, mock_shutil_which):
        with mock_shutil_which("gum"):
            app = GumBackupApp()
            assert app.gum_exe == "gum"

    def test_gum_not_found(self, mock_shutil_which):
        with mock_shutil_which(None):
            app = GumBackupApp()
            assert app.gum_exe is None


class TestFindGum:
    def test_find_gum_in_path(self, mock_shutil_which):
        with mock_shutil_which("gum"):
            app = GumBackupApp()
            result = app._find_gum()
            assert result == "gum"

    def test_find_gum_local_file(self, mock_shutil_which):
        with mock_shutil_which(None):
            with patch('os.path.exists', return_value=True):
                app = GumBackupApp()
                result = app._find_gum()
                assert result is not None

    def test_find_gum_not_found(self, mock_shutil_which):
        with mock_shutil_which(None):
            with patch('os.path.exists', return_value=False):
                app = GumBackupApp()
                result = app._find_gum()
                assert result is None


class TestCheckAdbWindows:
    def test_check_adb_windows_adb_present(self, mock_os_name, mock_shutil_which):
        with mock_os_name('nt'), mock_shutil_which("adb"):
            app = GumBackupApp()
            app.gum_exe = None
            
            app._check_and_install_adb_windows()

    def test_check_adb_windows_adb_not_found_yes(self, mock_os_name, mock_shutil_which):
        with mock_os_name('nt'), mock_shutil_which(None):
            app = GumBackupApp()
            app.gum_exe = None
            
            with patch.object(app, '_run_gum') as mock_gum:
                mock_gum.return_value = MagicMock(returncode=0, stdout="")
                
                with patch('subprocess.run') as mock_subprocess:
                    mock_subprocess.return_value = MagicMock(returncode=0, stderr="")
                    
                    with pytest.raises(SystemExit) as exc:
                        app._check_and_install_adb_windows()
                    
                    assert exc.value.code == 0

    def test_check_adb_windows_adb_not_found_no(self, mock_os_name, mock_shutil_which):
        with mock_os_name('nt'), mock_shutil_which(None):
            app = GumBackupApp()
            app.gum_exe = None
            
            with patch.object(app, '_run_gum') as mock_gum:
                mock_gum.return_value = MagicMock(returncode=1, stdout="")
                
                with pytest.raises(SystemExit) as exc:
                    app._check_and_install_adb_windows()
                
                assert exc.value.code == 1


class TestCheckAdbMacos:
    def test_check_adb_macos_adb_present(self, mock_os_name, mock_shutil_which):
        with mock_os_name('posix'), mock_shutil_which("adb"):
            with mock_shutil_which("brew"):
                app = GumBackupApp()
                app.gum_exe = None
                
                app._check_and_install_adb_macos()

    def test_check_adb_macos_homebrew_not_found_yes(self, mock_os_name, mock_shutil_which):
        def which_side_effect(cmd):
            if cmd == "brew":
                return None
            if cmd == "adb":
                return None
            return None
        
        with mock_os_name('posix'):
            with patch('shutil.which', side_effect=which_side_effect):
                app = GumBackupApp()
                app.gum_exe = None
                
                with patch.object(app, '_run_gum') as mock_gum:
                    mock_gum.return_value = MagicMock(returncode=0, stdout="")
                    
                    with patch('subprocess.run') as mock_subprocess:
                        mock_subprocess.return_value = MagicMock(returncode=0, stderr="")
                        
                        with pytest.raises(SystemExit) as exc:
                            app._check_and_install_adb_macos()
                        
                        assert exc.value.code == 0

    def test_check_adb_macos_homebrew_not_found_no(self, mock_os_name, mock_shutil_which):
        def which_side_effect(cmd):
            if cmd == "brew":
                return None
            return None
        
        with mock_os_name('posix'):
            with patch('shutil.which', side_effect=which_side_effect):
                app = GumBackupApp()
                app.gum_exe = None
                
                with patch.object(app, '_run_gum') as mock_gum:
                    mock_gum.return_value = MagicMock(returncode=1, stdout="")
                    
                    with pytest.raises(SystemExit) as exc:
                        app._check_and_install_adb_macos()
                    
                    assert exc.value.code == 1

    def test_check_adb_macos_adb_not_found_yes(self, mock_os_name, mock_shutil_which):
        def which_side_effect(cmd):
            if cmd == "brew":
                return "/usr/local/bin/brew"
            if cmd == "adb":
                return None
            return None
        
        with mock_os_name('posix'):
            with patch('shutil.which', side_effect=which_side_effect):
                app = GumBackupApp()
                app.gum_exe = None
                
                with patch.object(app, '_run_gum') as mock_gum:
                    mock_gum.return_value = MagicMock(returncode=0, stdout="")
                    
                    with patch('subprocess.run') as mock_subprocess:
                        mock_subprocess.return_value = MagicMock(returncode=0, stderr="")
                        
                        with pytest.raises(SystemExit) as exc:
                            app._check_and_install_adb_macos()
                        
                        assert exc.value.code == 0


class TestCheckAndInstallAdb:
    def test_check_and_install_adb_windows(self, mock_os_name, mock_shutil_which):
        with mock_os_name('nt'), mock_shutil_which("adb"):
            app = GumBackupApp()
            app.gum_exe = None
            
            app._check_and_install_adb()

    def test_check_and_install_adb_macos(self, mock_os_name, mock_shutil_which):
        with mock_os_name('posix'), mock_shutil_which("adb"):
            app = GumBackupApp()
            app.gum_exe = None
            
            app._check_and_install_adb()


class TestRunMethod:
    def test_run_calls_steps_in_order(self, mock_os_name, mock_shutil_which):
        with mock_os_name('posix'), mock_shutil_which("adb"):
            app = GumBackupApp()
            app.gum_exe = None
            
            with patch.object(app, 'step_select_mode') as mock_step1, \
                 patch.object(app, 'step1_select_filters') as mock_step2, \
                 patch.object(app, 'step2_select_drive') as mock_step3, \
                 patch.object(app, 'step3_scan_and_confirm') as mock_scan, \
                 patch.object(app, 'step4_perform_backup') as mock_backup:
                
                mock_scan.return_value = ([], 0)
                
                app.run()
                
                mock_step1.assert_called_once()
                mock_step2.assert_called_once()
                mock_step3.assert_called_once()
                mock_scan.assert_called_once()
                mock_backup.assert_called_once()

    def test_run_checks_adb_first(self, mock_os_name, mock_shutil_which):
        with mock_os_name('posix'), mock_shutil_which("adb"):
            app = GumBackupApp()
            app.gum_exe = None
            
            with patch.object(app, '_check_and_install_adb') as mock_check, \
                 patch.object(app, 'step_select_mode'), \
                 patch.object(app, 'step1_select_filters'), \
                 patch.object(app, 'step2_select_drive'), \
                 patch.object(app, 'step3_scan_and_confirm', return_value=([], 0)), \
                 patch.object(app, 'step4_perform_backup'):
                
                app.run()
                
                mock_check.assert_called_once()


class TestPrintHeader:
    def test_print_header_with_gum(self):
        app = GumBackupApp()
        app.gum_exe = "gum"
        
        with patch.object(app, '_run_gum') as mock_gum, \
             patch.object(app, 'clear_screen'):
            app.print_header("Test Title")
            
            mock_gum.assert_called_once()

    def test_print_header_without_gum(self):
        app = GumBackupApp()
        app.gum_exe = None
        
        with patch('os.system'):
            app.print_header("Test Title")


class TestClearScreen:
    def test_clear_screen(self):
        app = GumBackupApp()
        
        with patch('os.system') as mock_system:
            app.clear_screen()
            
            mock_system.assert_called_once()