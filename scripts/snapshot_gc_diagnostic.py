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
for snapshot in [False, True]:
    env = base.copy()
    if snapshot:
        env['TURBO_DOM_SNAPSHOT_PATH'] = '1'
    mode = 'snapshot' if snapshot else 'ordinary'
    cases = [(f'isolated-{index}', 'installed_document_without_custom_definitions_releases_context', '1') for index in range(8)]
    cases += [('custom-elements-single', 'browser_env::custom_elements::tests::', '1'),
              ('custom-elements-eight', 'browser_env::custom_elements::tests::', '8')]
    if snapshot:
        cases.append(('full-library-default-threads', '', None))
    cases += [('repeat-original-same-process', 'context_repeated_scope_release_diagnostic', '1'),
              ('actual-job-root-controls', 'context_collection_job_roots_diagnostic', '1')]
    for label, test_filter, threads in cases:
        command = [binaries[0]]
        if test_filter:
            command.append(test_filter)
        if threads:
            command.append('--test-threads=' + threads)
        command.append('--nocapture')
        name = mode + '-' + label
        started = time.monotonic()
        env['TURBO_DOM_GC_DIAGNOSTIC_DIR'] = str((args.out / (name + '-roots')).resolve())
        with (args.out / (name + '.log')).open('wb') as log:
            try:
                result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=240)
                code = result.returncode
            except subprocess.TimeoutExpired:
                code = 'timeout'
        record = dict(name=name, command=command, exit=code, seconds=time.monotonic() - started)
        records.append(record)
        (args.out / 'summary.json').write_text(json.dumps(records, indent=2) + '\n')
        print(json.dumps(record), flush=True)
env = base.copy()
env['TURBO_DOM_SNAPSHOT_PATH'] = '1'
env['TURBO_DOM_DIAGNOSTIC_FULL_GC'] = '1'
env['TURBO_DOM_GC_DIAGNOSTIC_DIR'] = str((args.out / 'testing-full-roots').resolve())
with (args.out / 'testing-full-controls.log').open('wb') as log:
    result = subprocess.run([binaries[0], 'context_collection_job_roots_diagnostic', '--test-threads=1', '--nocapture'], env=env, stdout=log, stderr=subprocess.STDOUT, timeout=120)
records.append(dict(name='testing-full-controls-fresh-process', exit=result.returncode))
(args.out / 'summary.json').write_text(json.dumps(records, indent=2) + '\n')
raise SystemExit(1 if any(record['exit'] != 0 for record in records) else 0)
