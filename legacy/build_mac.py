import PyInstaller.__main__
import os
import shutil
import shutil as _shutil

def build():
    if os.path.exists('build'):
        shutil.rmtree('build')
    if os.path.exists('dist'):
        shutil.rmtree('dist')
    gum_path = _shutil.which('gum')
    args = [
        'main.py',
        '--name=AutoBackupMAC',
        '--onefile',
        '--console',
        '--icon=NONE',
        '--clean',
        '--add-data=backup_engine.py:.'
    ]
    if gum_path and os.path.exists(gum_path):
        args.append(f'--add-binary={gum_path}:.')
    PyInstaller.__main__.run(args)

if __name__ == "__main__":
    build()
