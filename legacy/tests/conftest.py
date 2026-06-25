import pytest
import os
import sys
from unittest.mock import patch, MagicMock

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


@pytest.fixture
def mock_os_name():
    def _mock(name):
        return patch('os.name', name)
    return _mock


@pytest.fixture
def mock_platform():
    def _mock(system):
        return patch('platform.system', return_value=system)
    return _mock


@pytest.fixture
def mock_shutil_which():
    def _mock(return_value):
        return patch('shutil.which', return_value=return_value)
    return _mock


@pytest.fixture
def mock_subprocess():
    return patch('subprocess.run')


@pytest.fixture
def mock_psutil():
    psutil_mock = MagicMock()
    psutil_mock.disk_partitions.return_value = []
    psutil_mock.disk_usage.return_value = MagicMock(total=1000000000, free=500000000)
    return patch('backup_engine.psutil', psutil_mock)


@pytest.fixture
def temp_dir(tmp_path):
    return tmp_path


@pytest.fixture
def mock_gum_exe():
    return patch('main.GumBackupApp._run_gum', return_value=MagicMock(returncode=0, stdout=""))


class MockGumResult:
    def __init__(self, returncode=0, stdout=""):
        self.returncode = returncode
        self.stdout = stdout


@pytest.fixture
def gum_result():
    return MockGumResult