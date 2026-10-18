"""Opt-in volunteer node registry. No remote execution or scanning."""
import argparse, json, sqlite3, pathlib, datetime
DB=pathlib.Path(__file__).with_name('hunter.db')
def init(c):
 c.execute('CREATE TABLE IF NOT EXISTS volunteers (node_id TEXT PRIMARY KEY, gpu TEXT, vram_gb REAL, owner_consent INTEGER, allowed_workloads TEXT, max_hours_day REAL, status TEXT, registered_utc TEXT)')
def register(node_id,gpu,vram,consent,workloads,max_hours):
 if not consent: raise ValueError('Explicit owner consent required')
 if not node_id or vram<=0 or not (0<max_hours<=24): raise ValueError('Invalid node limits')
 if not workloads or any(w not in {'inference','benchmark','render','research'} for w in workloads): raise ValueError('Invalid workload permissions')
 with sqlite3.connect(DB) as c:
  init(c);c.execute('INSERT INTO volunteers VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(node_id) DO UPDATE SET gpu=excluded.gpu,vram_gb=excluded.vram_gb,owner_consent=excluded.owner_consent,allowed_workloads=excluded.allowed_workloads,max_hours_day=excluded.max_hours_day,status=excluded.status',(node_id,gpu,vram,1,','.join(workloads),max_hours,'available',datetime.datetime.now(datetime.timezone.utc).isoformat()))
 return {'node_id':node_id,'status':'registered_opt_in','remote_execution':False}
def match(vram,workload):
 with sqlite3.connect(DB) as c:
  init(c);rows=c.execute('SELECT node_id,gpu,vram_gb,allowed_workloads FROM volunteers WHERE owner_consent=1 AND status="available" AND vram_gb>=?',(vram,)).fetchall()
 return [{'node_id':a,'gpu':b,'vram_gb':c} for a,b,c,d in rows if workload in d.split(',')]
if __name__=='__main__':
 p=argparse.ArgumentParser();s=p.add_subparsers(dest='cmd',required=True)
 r=s.add_parser('register');r.add_argument('--node-id',required=True);r.add_argument('--gpu',required=True);r.add_argument('--vram',type=float,required=True);r.add_argument('--owner-consent',action='store_true');r.add_argument('--workloads',nargs='+',default=['inference','benchmark']);r.add_argument('--max-hours',type=float,default=2)
 m=s.add_parser('match');m.add_argument('--vram',type=float,required=True);m.add_argument('--workload',required=True)
 a=p.parse_args();print(json.dumps(register(a.node_id,a.gpu,a.vram,a.owner_consent,a.workloads,a.max_hours) if a.cmd=='register' else match(a.vram,a.workload),indent=2))
