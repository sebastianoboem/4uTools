import PyInstaller.__main__
import os
import shutil

def build():
    # Clean previous builds
    if os.path.exists('build'):
        shutil.rmtree('build')
    if os.path.exists('dist'):
        shutil.rmtree('dist')

    print("Avvio creazione eseguibile portable...")

    gum_path = 'gum.exe'
    if not os.path.exists(gum_path):
        print("ATTENZIONE: 'gum.exe' non trovato nella cartella corrente!")
        print("Per creare l'eseguibile portable funzionante, scarica 'gum.exe' da https://github.com/charmbracelet/gum/releases")
        print("e posizionalo in questa cartella.")
        input("Premi Invio per continuare comunque (la build potrebbe fallire o non funzionare)...")

    args = [
        'main.py',
        '--name=AutoBackupPC-ANDROID',
        '--onefile',
        '--console',
        '--icon=NONE',
        '--clean',
        '--add-data=backup_engine.py;.'
    ]
    
    if os.path.exists(gum_path):
        args.append(f'--add-binary={gum_path};.')

    PyInstaller.__main__.run(args)
    
    print("Build completata! L'eseguibile si trova nella cartella 'dist'.")

if __name__ == "__main__":
    build()
