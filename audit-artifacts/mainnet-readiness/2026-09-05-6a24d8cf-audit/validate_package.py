"""Validate audit data, rendered text/navigation, source preservation and artifacts."""
from pathlib import Path
import collections,csv,datetime,hashlib,json,re,subprocess,tempfile,xml.etree.ElementTree as ET
R=Path(__file__).resolve().parent
repo=R.parents[2]
def read(p):return json.loads((R/p).read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 s=read('scorecard.json');fs=read('findings.json')['findings'];d=read('report-source.json');nav=read('pdf-navigation.json');fig=read('figure-register.json');pro=read('provenance.json')
 assert len(fs)==len({f['id'] for f in fs})==29
 assert dict(collections.Counter(f['status'] for f in s['features']))==s['feature_counts']
 assert len(s['features'])==64
 for f in s['features']:assert f['completion_percent']==20*sum(f[k] for k in ('implemented','wired','tested','executed','reproducible'))
 assert sum(x['weight'] for x in s['subsystems'])==100
 raw=0
 for x in s['subsystems']:
  subset=[f for f in s['features'] if f['id'] in x['feature_ids']];value=sum(f['completion_percent'] for f in subset)/len(subset)
  assert abs(value-x['raw_score'])<.011
  raw+=value*x['weight']/100
 assert round(raw,2)==s['uncapped_score'] and s['readiness_score']==min(raw,20)
 with (R/'feature-completeness.csv').open() as f:assert len(list(csv.DictReader(f)))==64
 refs=[b for sec in d['sections'] for b in sec['blocks'] if b['type']=='figure']
 assert {x['id'] for x in refs}=={x['id'] for x in fig}
 for x in refs:
  assert x['caption'] and (R/'assets'/f"{x['id']}.svg").is_file() and (R/'assets'/f"{x['id']}.pdf").is_file()
 with tempfile.TemporaryDirectory() as td:
  xml=Path(td)/'bbox.html';subprocess.run(['pdftotext','-bbox',str(R/'X3-ROAD-TO-MAINNET.pdf'),str(xml)],check=True)
  pages=ET.parse(xml).getroot().findall('.//{*}page');outside=[];glyph=[];sparse=[]
  texts=[]
  for n,p in enumerate(pages,1):
   words=p.findall('.//{*}word');texts.append(' '.join(w.text or '' for w in words))
   if len(words)<20:sparse.append(n)
   for w in words:
    if float(w.attrib['xMin'])<0 or float(w.attrib['yMin'])<0 or float(w.attrib['xMax'])>float(p.attrib['width'])+.1 or float(w.attrib['yMax'])>float(p.attrib['height'])+.1:outside.append(n)
    if '\ufffd' in (w.text or ''):glyph.append(n)
  assert not outside and not glyph and len(pages)==nav['page_count']
  toc=' '.join(texts[2:4]);titles=[]
  for x in nav['chapter_pages']:
   assert x['title'] in texts[x['page']-1],x
   assert x['title'] in toc,x
   titles.append({'title':x['title'],'page':x['page'],'heading_and_toc_text_present':True})
 pdf=(R/'X3-ROAD-TO-MAINNET.pdf').read_bytes()
 objects=dict(re.findall(rb'(\d+) 0 obj\s*(.*?)endobj',pdf,re.S))
 destinations=re.findall(rb'/Dest\s*\[\s*(\d+) 0 R',pdf)
 assert destinations and all(re.search(rb'/Type\s*/Page\b',objects.get(n,b'')) for n in destinations)
 # Compare exactly the same git-status mode used at the start, excluding audit artifacts.
 status=subprocess.check_output(['git','status','--porcelain=v1','--untracked-files=normal'],cwd=repo,text=True)
 (R/'evidence/working-tree-after.txt').write_text(status)
 norm=lambda t:sorted(x for x in t.splitlines() if 'audit-artifacts/' not in x)
 unchanged=norm(status)==norm((R/'evidence/working-tree-before.txt').read_text())
 hashes=read('evidence/source-hashes.json');changed=[x['path'] for x in hashes if not (repo/x['path']).is_file() or sha(repo/x['path'])!=x['sha256']]
 lockchanged=[p for p,h in pro['lockfile_hashes'].items() if sha(repo/p)!=h]
 assert unchanged and not changed and not lockchanged
 # High-confidence credential shapes only; values are never included in the result.
 pats=[rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----',rb'AKIA[0-9A-Z]{16}',rb'gh[pousr]_[A-Za-z0-9]{36,}',rb'github_pat_[A-Za-z0-9_]{50,}',rb'xox[baprs]-[A-Za-z0-9-]{20,}']
 hits=[];scanned=0
 for p in R.rglob('*'):
  if not p.is_file() or '__pycache__' in p.parts or p.name in ('validate_package.py','artifact-validation.json','manifest.json','manifest.sha256') or p.suffix in ('.pdf','.png','.jpg'):continue
  content=p.read_bytes();scanned+=1
  if any(re.search(pattern,content) for pattern in pats):hits.append(str(p.relative_to(R)))
 assert not hits,hits
 result={'checked_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':'/usr/bin/python3 validate_package.py','exit_code':0,'pdf_pages':len(pages),'internal_pdf_destinations_resolved':len(destinations),'source_files_checked':len(hashes),'source_hash_mismatches':changed,'lockfile_hash_mismatches':lockchanged,'working_tree_status_equal_excluding_audit_artifacts':unchanged,'score_formula_and_counts_consistent':True,'figure_assets_and_references_checked':len(fig),'words_outside_page':outside,'replacement_glyphs':glyph,'pages_with_fewer_than_20_words':sparse,'navigation_checks':titles,'credential_pattern_scanned_files':scanned,'credential_pattern_match_paths':hits,'credential_scan_limit':'Heuristic pattern scan plus review of generated report/log content. Not a guarantee against every secret format; sensitive input files were excluded.','visual_inspection':'All final pages rendered to PNG; contact sheets and selected full-size pages reviewed. Vector figures; no observed clipping or overlaps.','render_repairs':['Grouped closing statement and compacted final explanatory notes to remove isolated overflow lines.','Replaced unavailable Drawing.scaled with copied/scaled vector drawing.','Compressed TOC spacing to avoid one-entry overflow; separate table list page; reduced body/table spacing to improve pagination.']}
 (R/'artifact-validation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='navigation_checks'},indent=2))
if __name__=='__main__':main()
