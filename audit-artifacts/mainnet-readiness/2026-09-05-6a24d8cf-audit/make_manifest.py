"""Generate or verify per-file audit package integrity without circular hashes."""
from pathlib import Path
import datetime,hashlib,json,sys
R=Path(__file__).resolve().parent
skip={'manifest.json','manifest.sha256'}
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def purpose(p):
 if p.parts[0]=='visual-review':return 'Contact-sheet evidence from all-page raster layout review'
 if p.parts[0]=='assets':return 'Reusable vector audit figure'
 if p.parts[0]=='audit-harness':return 'Reproducible bounded audit test harness or locked dependencies'
 if p.parts[0]=='evidence':return 'Retained audit evidence, command log or repository inventory'
 return {'X3-ROAD-TO-MAINNET.pdf':'Final illustrated field manual','source.md':'Full Markdown manuscript','README.md':'Artifact index and reproduction instructions','findings.json':'Detailed findings register','feature-completeness.csv':'Capability acceptance matrix','executive-summary.md':'Standalone executive summary','scorecard.json':'Evidence criteria, weighting and score calculation','report-source.json':'Structured report rendering source','provenance.json':'Audited source provenance and exclusions','recovery-plan.json':'Phased remediation backlog','launch-gates.json':'Proposed objective launch gates','benchmark-results.csv':'Blank unmeasured performance template','figure-register.json':'Figure numbering and captions','pdf-navigation.json':'Rendered chapter page and table register','artifact-validation.json':'Final document, data and source-preservation validation'}.get(p.name,'Maintainable audit package generation or validation source')
def main():
 if '--verify' in sys.argv:
  m=json.loads((R/'manifest.json').read_text());bad=[x['path'] for x in m['files'] if not (R/x['path']).is_file() or digest(R/x['path'])!=x['sha256']]
  actual={str(p.relative_to(R)) for p in R.rglob('*') if p.is_file() and p.name not in skip and '__pycache__' not in p.parts}
  assert actual=={x['path'] for x in m['files']},'Manifest file set differs'
  assert not bad,bad
  assert (R/'manifest.sha256').read_text().split()[0]==digest(R/'manifest.json')
  print(f"PASS: {len(m['files'])} file hashes and detached manifest checksum");return
 pro=json.loads((R/'provenance.json').read_text());entries=[]
 for p in sorted(R.rglob('*')):
  if not p.is_file() or p.name in skip or '__pycache__' in p.parts:continue
  stat=p.stat();created=getattr(stat,'st_birthtime',stat.st_mtime)
  entries.append({'path':str(p.relative_to(R)),'purpose':purpose(p.relative_to(R)),'created_at_utc':datetime.datetime.fromtimestamp(created,datetime.timezone.utc).isoformat(),'creation_time_basis':'filesystem birth time when available; otherwise last modification time','repository_commit_sha':pro['commit'],'bytes':stat.st_size,'sha256':digest(p)})
 m={'generated_at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'repository_commit_sha':pro['commit'],'audited_tree':'Base commit plus pre-existing working-tree changes; see provenance.json','hash_algorithm':'SHA-256','pdf_pages':json.loads((R/'pdf-navigation.json').read_text())['page_count'],'excluded':['manifest.json (detached checksum in manifest.sha256)','manifest.sha256','__pycache__/'],'files':entries}
 (R/'manifest.json').write_text(json.dumps(m,indent=2)+'\n');(R/'manifest.sha256').write_text(digest(R/'manifest.json')+'  manifest.json\n');print('Manifest files:',len(entries))
if __name__=='__main__':main()
