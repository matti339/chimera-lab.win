#!/bin/zsh
set -eu
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
python3 - "$script_dir" <<'PY'
import datetime, pathlib, shutil, subprocess, sys, tempfile, os
source = pathlib.Path(sys.argv[1]) / 'Chimera Lab.vst3'
if not source.is_dir():
    raise SystemExit('Keep this installer beside Chimera Lab.vst3.')
subprocess.run(['codesign', '--verify', '--strict', str(source)], check=True)
plugins = pathlib.Path.home() / 'Library/Audio/Plug-Ins'
folder = plugins / 'VST3'
folder.mkdir(parents=True, exist_ok=True)
destination = folder / source.name
stage = pathlib.Path(tempfile.mkdtemp(prefix='.chimera-update-', dir=folder)) / source.name
shutil.copytree(source, stage)
subprocess.run(['codesign', '--verify', '--strict', str(stage)], check=True)
backup = None
if destination.exists():
    backup = plugins / 'Chimera Lab Backups' / datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f') / source.name
    backup.parent.mkdir(parents=True)
    os.rename(destination, backup)
try:
    os.rename(stage, destination)
except Exception:
    if backup is not None:
        os.rename(backup, destination)
    raise
stage.parent.rmdir()
print('Installed Chimera Lab 0.4. Restart Live to load the update; rescan VST3 plugins if needed.')
if backup is not None:
    print('Previous build preserved at:', backup)
PY
