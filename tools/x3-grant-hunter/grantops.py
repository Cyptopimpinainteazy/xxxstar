"""X3 grant opportunity CRM: local-only, no auto-submission or spending."""
import argparse
import datetime as dt
import json
import pathlib
import sqlite3
from urllib.parse import urlparse

DB = pathlib.Path(__file__).with_name('hunter.db')
STAGES = ('discovered','researching','eligible_review','drafting','approval_pending','approved','submitted','follow_up','awarded','rejected','closed')

def connect(path=None):
    c = sqlite3.connect(path or DB)
    c.row_factory = sqlite3.Row
    c.execute('''CREATE TABLE IF NOT EXISTS grant_pipeline (
      id TEXT PRIMARY KEY, name TEXT NOT NULL, official_url TEXT NOT NULL,
      category TEXT NOT NULL, amount_usd REAL, deadline TEXT,
      entity_required INTEGER NOT NULL DEFAULT 0,
      evidence_status TEXT NOT NULL DEFAULT 'unverified',
      stage TEXT NOT NULL DEFAULT 'discovered',
      approval_record TEXT, notes TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL)''')
    return c

def import_leads(leads, path=None):
    results=[]
    with connect(path) as c:
        for x in leads:
            if not isinstance(x,dict) or not all(isinstance(x.get(k),str) and x[k].strip() for k in ('id','name','official_url','category')):
                raise ValueError('missing required lead fields')
            u=urlparse(x['official_url'])
            if u.scheme!='https' or not u.hostname or u.username or u.password:
                raise ValueError('official_url must be a public HTTPS URL')
            deadline=x.get('deadline')
            if deadline: dt.date.fromisoformat(deadline)
            amount=x.get('amount_usd')
            if amount is not None and (isinstance(amount,bool) or not isinstance(amount,(int,float)) or not 0<=amount<1e12):
                raise ValueError('invalid amount')
            now=dt.datetime.now(dt.timezone.utc).isoformat()
            c.execute('''INSERT INTO grant_pipeline(id,name,official_url,category,amount_usd,deadline,entity_required,notes,updated_at)
              VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET
              name=excluded.name, official_url=excluded.official_url, category=excluded.category,
              amount_usd=excluded.amount_usd, deadline=excluded.deadline,
              entity_required=excluded.entity_required, notes=excluded.notes,updated_at=excluded.updated_at''',
              (x['id'],x['name'],x['official_url'],x['category'],amount,deadline,int(bool(x.get('entity_required'))),x.get('notes',''),now))
            results.append(x['id'])
    return results

def score(row, company='forming', today=None):
    """Prioritization only; not a statement of eligibility or grant availability."""
    today=today or dt.date.today()
    if row['entity_required'] and company!='incorporated':
        return {'score':0,'gate':'incorporation_review_required'}
    deadline=row['deadline']
    if deadline and dt.date.fromisoformat(deadline)<today:
        return {'score':0,'gate':'deadline_passed_verify_reopening'}
    category=row['category'].lower()
    points={'gpu':85,'cloud':80,'blockchain':80,'security':75,'research':65,'other':45}.get(category,40)
    if deadline and (dt.date.fromisoformat(deadline)-today).days<=14: points+=5
    return {'score':min(points,100),'gate':'manual_eligibility_and_terms_review'}

def pipeline(company='forming',path=None):
    with connect(path) as c:
        return [{**dict(r),**score(r,company)} for r in c.execute('SELECT * FROM grant_pipeline ORDER BY updated_at DESC')]

def move(lead_id, stage, approval_record=None, path=None):
    if stage not in STAGES: raise ValueError('invalid stage')
    if stage in ('approved','submitted','awarded') and not (isinstance(approval_record,str) and approval_record.strip()):
        raise ValueError('human approval/evidence reference required')
    with connect(path) as c:
        row=c.execute('SELECT id,approval_record FROM grant_pipeline WHERE id=?',(lead_id,)).fetchone()
        if row is None: raise ValueError('unknown opportunity')
        if stage=='submitted' and not (approval_record or row['approval_record']):
            raise ValueError('approval record required')
        c.execute('UPDATE grant_pipeline SET stage=?,approval_record=COALESCE(?,approval_record),updated_at=? WHERE id=?',
                  (stage,approval_record,dt.datetime.now(dt.timezone.utc).isoformat(),lead_id))
    return {'id':lead_id,'stage':stage,'no_external_action':True}

def main():
    p=argparse.ArgumentParser(); p.add_argument('--db',type=pathlib.Path)
    sub=p.add_subparsers(dest='cmd',required=True)
    i=sub.add_parser('import');i.add_argument('json_file',type=pathlib.Path)
    l=sub.add_parser('list');l.add_argument('--company',choices=['forming','incorporated','individual'],default='forming')
    m=sub.add_parser('move');m.add_argument('id');m.add_argument('stage',choices=STAGES);m.add_argument('--approval-record')
    a=p.parse_args()
    result=import_leads(json.loads(a.json_file.read_text()),a.db) if a.cmd=='import' else pipeline(a.company,a.db) if a.cmd=='list' else move(a.id,a.stage,a.approval_record,a.db)
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
