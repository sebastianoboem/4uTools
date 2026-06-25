#!/bin/bash

# Directory dello script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Nome del venv
VENV_DIR=".venv"

# Controllo se gum è installato
if ! command -v gum &> /dev/null; then
    echo "⚠️  'gum' non trovato."
    echo "Per eseguire questo script è necessario installare 'gum'."
    echo "Su macOS puoi installarlo con Homebrew:"
    echo "  brew install gum"
    exit 1
fi

# Creazione venv se non esiste
if [ ! -d "$VENV_DIR" ]; then
    echo "📦 Creazione ambiente virtuale (.venv)..."
    python3 -m venv "$VENV_DIR"
fi

# Attivazione
source "$VENV_DIR/bin/activate"

# Installazione dipendenze
echo "⬇️  Verifica dipendenze..."
if [ -f "requirements.txt" ]; then
    pip install -r requirements.txt --quiet
else
    echo "⚠️  File requirements.txt non trovato!"
    exit 1
fi

# Avvio build
echo "🔨 Avvio Build macOS..."
python build_mac.py

echo "✅ Build completata. Eseguibile in: dist/AutoBackupMAC"
