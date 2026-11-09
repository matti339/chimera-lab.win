#!/usr/bin/env python3
"""Package the compiled macOS library as an ad-hoc signed VST3 bundle."""
import pathlib, plistlib, shutil, subprocess, sys
root = pathlib.Path(__file__).resolve().parents[2]
out = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / 'target/bundled'
bundle = out / 'Chimera Lab.vst3'
contents = bundle / 'Contents'
(contents / 'MacOS').mkdir(parents=True, exist_ok=True)
shutil.copy2(root / 'target/release/libchimera_plugin.dylib', contents / 'MacOS/Chimera Lab')
with (contents / 'Info.plist').open('wb') as f:
    plistlib.dump(dict(CFBundleExecutable='Chimera Lab', CFBundleIdentifier='org.chimera-community.lab',
        CFBundleName='Chimera Lab', CFBundleDisplayName='Chimera Lab', CFBundlePackageType='BNDL',
        CFBundleSignature='????', CFBundleShortVersionString='0.4.0', CFBundleVersion='0.4.0',
        NSHighResolutionCapable=True), f)
(contents / 'PkgInfo').write_text('BNDL????')
subprocess.run(['codesign', '--force', '--sign', '-', str(bundle)], check=True)
subprocess.run(['codesign', '--verify', '--strict', str(bundle)], check=True)
print(bundle)
