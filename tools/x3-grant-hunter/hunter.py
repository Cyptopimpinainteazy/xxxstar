import argparse, datetime as dt, json, pathlib, sqlite3, urllib.request
DB=pathlib.Path(__file__).with_name('hunter.db')
SOURCES=[{'id':'google-start','name':'Google for Startups Start','url':'https://cloud.google.com/startup/pre-funded','kind':'credit','requires_company':True,'gpu_supported':'verify','notes':'Working MVP, business model, funding plans; mining excluded.'},{'id':'hf-zerogpu','name':'Hugging Face ZeroGPU','url':'https://huggingface.co/docs/hub/spaces-zerogpu','kind':'free_gpu','requires_company':False,'gpu_supported':'yes','notes':'Gradio/PyTorch use, per-user quotas; no arbitrary VM.'},{'id':'hf-community','name':'Hugging Face Community GPU Grants','url':'https://huggingface.co/docs/hub/spaces-gpus','kind':'grant','requires_company':False,'gpu_supported':'yes','notes':'Request support for an eligible public Space.'}]
def connect():
 c=sqlite3.connect(DB);c.execute('create table if not exists opportunities(id text primary key,name text,url text,kind text,status text,notes text,last_checked text)');c.execute('create table if not exists jobs(id integer primary key,workload text,provider text,status text,estimated_cost real,created text)');return c
def qualify(s,company):
 return 'blocked_pending_incorporation' if s['requires_company'] and company!='incorporated' else 'candidate_needs_review'
def discover(company='forming',fetch=False):
 c=connect();out=[]
 for s in SOURCES:
  status=qualify(s,company);notes=s['notes']
  if fetch:
   try:
    req=urllib.request.Request(s['url'],headers={'User-Agent':'X3GPUHunter/0.1 (opportunity metadata checker)'})
    with urllib.request.urlopen(req,timeout=8) as r: notes+=' HTTP '+str(r.status)
   except Exception as e: notes+=' Reachability not verified: '+type(e).__name__
  c.execute('insert into opportunities values(?,?,?,?,?,?,?) on conflict(id) do update set status=excluded.status,notes=excluded.notes,last_checked=excluded.last_checked',(s['id'],s['name'],s['url'],s['kind'],status,notes,dt.datetime.now(dt.timezone.utc).isoformat()))
  out.append({'id':s['id'],'status':status,'url':s['url']})
 c.commit();c.close();return out
def draft(id):
 s=next((x for x in SOURCES if x['id']==id),None)
 if not s: raise ValueError('Unknown opportunity')
 p=pathlib.Path(__file__).with_name('drafts');p.mkdir(exist_ok=True)
 f=p/(id+'.md');f.write_text(f'''# X3 application draft — {s['name']}\n\nStatus: DRAFT ONLY — do not submit automatically.\nProgram: {s['url']}\n\n## Project\nX3 is developing blockchain infrastructure, X3Lang developer tooling, and a GPU compute orchestration system.\n\n## Proposed compute use\nGPU-backed model evaluation, inference prototypes, and reproducible AI developer tooling benchmarks. No cryptocurrency mining.\n\n## Evidence required before submission\n- Legal entity name, incorporation date, company website and email\n- Public demo, repository links, roadmap, and current product maturity\n- Requested GPU type, expected hours, estimated cost and research deliverables\n- Program-specific eligibility, token-related restrictions, and authorized signatory\n\n## Questions to verify\n{s['notes']}\n\nNo claims of funding, incorporation, approved credits, or guaranteed GPUs are made.\n''');return str(f)
def allocate(workload,provider='hf-zerogpu',estimated_cost=0):
 if estimated_cost!=0: raise ValueError('Zero-spend policy: paid workloads rejected')
 if provider!='hf-zerogpu': raise ValueError('Provider not approved for zero-cost scheduling')
 c=connect();cur=c.execute('insert into jobs(workload,provider,status,estimated_cost,created) values(?,?,?,?,?)',(workload,provider,'awaiting_provider_auth_and_quota_check',0,dt.datetime.now(dt.timezone.utc).isoformat()));c.commit();id=cur.lastrowid;c.close();return {'job_id':id,'status':'awaiting_provider_auth_and_quota_check','note':'No remote job dispatched; free quota and authorized endpoint required.'}
def main():
 p=argparse.ArgumentParser();sub=p.add_subparsers(dest='command',required=True)
 d=sub.add_parser('discover');d.add_argument('--company',choices=['forming','incorporated','individual'],default='forming');d.add_argument('--fetch',action='store_true')
 a=sub.add_parser('draft');a.add_argument('id',choices=[s['id'] for s in SOURCES]);j=sub.add_parser('allocate');j.add_argument('workload');j.add_argument('--provider',default='hf-zerogpu');j.add_argument('--estimated-cost',type=float,default=0)
 args=p.parse_args();print(json.dumps(discover(args.company,args.fetch) if args.command=='discover' else {'file':draft(args.id)} if args.command=='draft' else allocate(args.workload,args.provider,args.estimated_cost),indent=2))
if __name__=='__main__':main()
