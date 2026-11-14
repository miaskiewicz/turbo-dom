"""Run unchanged native context-release tests; every failure stays a failure."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--build', type=Path, required=True)
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=True)
artifacts = [json.loads(line) for line in args.build.read_text().splitlines() if line.startswith('{')]
binaries = [item['executable'] for item in artifacts
            if item.get('reason') == 'compiler-artifact' and item.get('executable')
            and item.get('target', {}).get('name') == 'turbo_dom_browser_env'
            and item.get('profile', {}).get('test')]
if len(binaries) != 1:
    raise RuntimeError(f'Expected one native library test binary, got {binaries}')
base = os.environ.copy()
base.pop('TURBO_DOM_SNAPSHOT_PATH', None)
base.pop('TURBO_DOM_ATTACH_PATH', None)
records = []
for synchronous in [False, True]:
    env = base.copy()
    env['TURBO_DOM_SNAPSHOT_PATH'] = '1'
    flags = '--trace-concurrent-recompilation --trace-opt'
    if synchronous:
        flags += ' --no-concurrent-recompilation'
    env['TURBO_DOM_DIAGNOSTIC_COMPILER_FLAGS'] = flags
    mode = 'synchronous' if synchronous else 'concurrent'
    for index in range(8):
        name = f'{mode}-group-{index}'
        env['TURBO_DOM_GC_DIAGNOSTIC_DIR'] = str((args.out / (name + '-roots')).resolve())
        command = [binaries[0], 'browser_env::custom_elements::tests::', '--skip=context_repeated_scope_release_diagnostic', '--test-threads=8', '--nocapture']
        started = time.monotonic()
        with (args.out / (name + '.log')).open('wb') as log:
            try:
                result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=120)
                code = result.returncode
            except subprocess.TimeoutExpired:
                code = 'timeout'
        records.append(dict(name=name, command=command, flags=flags, exit=code, seconds=time.monotonic()-started))
        (args.out / 'summary.json').write_text(json.dumps(records, indent=2) + '\n')
        print(json.dumps(records[-1]), flush=True)
raise SystemExit(1 if any(record['exit'] != 0 for record in records) else 0)
