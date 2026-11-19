import argparse, json, os, resource, subprocess, time
from pathlib import Path
parser=argparse.ArgumentParser(description='Observe a Linux AVIF diagnostic process without changing resource limits')
parser.add_argument('binary')
parser.add_argument('receipt')
args=parser.parse_args()
start=time.monotonic()
p=subprocess.Popen([args.binary],env={**os.environ,'RUST_BACKTRACE':'full'})
peak_threads=peak_cgroup_pids=peak_sampled_rss=0
limits={name:Path('/sys/fs/cgroup/'+name).read_text().strip() for name in ['pids.max','memory.max']}
while p.poll() is None:
 try:
  status=dict(line.split(':',1) for line in Path(f'/proc/{p.pid}/status').read_text().splitlines() if ':' in line)
  peak_threads=max(peak_threads,int(status['Threads']))
  peak_sampled_rss=max(peak_sampled_rss,int(status.get('VmHWM','0 kB').split()[0]))
  peak_cgroup_pids=max(peak_cgroup_pids,int(Path('/sys/fs/cgroup/pids.current').read_text()))
 except (FileNotFoundError,ProcessLookupError): pass
 time.sleep(0.01)
r={'returncode':p.returncode,'wall_seconds':time.monotonic()-start,'peak_rss_kib':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,'sampled_peak_threads':peak_threads,'sampled_peak_cgroup_pids':peak_cgroup_pids,'sampled_peak_rss_kib':peak_sampled_rss,'limits_unchanged':limits,'sample_period_seconds':0.01}
Path(args.receipt).write_text(json.dumps(r,indent=2)+'\n')
print('CONCURRENCY-RESOURCE',json.dumps(r),flush=True)
raise SystemExit(0 if p.returncode==0 else 1)
