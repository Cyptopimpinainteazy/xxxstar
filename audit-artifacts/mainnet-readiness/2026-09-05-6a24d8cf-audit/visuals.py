"""Reusable vector audit figures. Values come from scorecard/findings JSON."""
from pathlib import Path
import json,math,collections
from reportlab.graphics.shapes import Drawing,Rect,Line,String,Polygon,Circle,PolyLine
from reportlab.graphics import renderSVG,renderPDF
from reportlab.lib.colors import HexColor,Color
ROOT=Path(__file__).resolve().parent
NAVY=HexColor('#142B40'); TEAL=HexColor('#007C83'); BLUE=HexColor('#2264A8'); RED=HexColor('#B22F3D'); AMBER=HexColor('#9A5D00'); GRAY=HexColor('#5F6F7A'); LIGHT=HexColor('#EEF3F6'); WHITE=HexColor('#FFFFFF')
DRAWS={}; CAPTIONS={}
def text(d,x,y,s,size=10,color=NAVY,anchor='start'):
 d.add(String(x,y,str(s),fontName='Helvetica',fontSize=size,fillColor=color,textAnchor=anchor))
def title(d,s): text(d,18,d.height-25,s,14)
def register(key,caption,d): DRAWS[key]=d;CAPTIONS[key]=caption

def bars(key,caption,labels,values,maxv=None,colors=None,unit='',note=''):
 h=85+len(labels)*26; d=Drawing(500,h);title(d,caption.split(' — ')[0]); maxv=maxv or max(values+[1]); x=220; width=225
 for i,(label,val) in enumerate(zip(labels,values)):
  y=h-60-i*26; text(d,12,y+2,label[:38],9); d.add(Rect(x,y-1,width,14,fillColor=LIGHT,strokeColor=None));d.add(Rect(x,y-1,width*val/maxv,14,fillColor=(colors[i] if colors else TEAL),strokeColor=None));text(d,450,y+2,f'{val:g}{unit}',9)
 text(d,12,14,note[:100],8,GRAY);register(key,caption,d)
def flow(key,caption,labels,edges=None,cols=3,states=None,note='Static wiring; no live deployment is asserted.'):
 rows=math.ceil(len(labels)/cols);h=rows*98+85; d=Drawing(500,h);title(d,caption);pos=[];w=(464-(cols-1)*16)/cols
 for i,label in enumerate(labels):
  c=i%cols;r=i//cols;x=18+c*(w+16);y=h-64-r*98-55;pos.append((x,y,w,61))
 if edges is None: edges=[(i,i+1) for i in range(len(labels)-1)]
 for a,b in edges:
  x,y,w0,h0=pos[a];xx,yy,ww,hh=pos[b]
  if a//cols==b//cols: p=(x+w0,y+h0/2,xx,yy+hh/2)
  else:p=(x+w0/2,y,xx+ww/2,yy+hh)
  d.add(Line(*p,strokeColor=GRAY,strokeWidth=1));vx,vy=p[2]-p[0],p[3]-p[1];m=max(math.hypot(vx,vy),1);ux,uy=vx/m,vy/m;ex,ey=p[2],p[3];d.add(Polygon([ex,ey,ex-6*ux+3*uy,ey-6*uy-3*ux,ex-6*ux-3*uy,ey-6*uy+3*ux],fillColor=GRAY,strokeColor=None))
 for i,label in enumerate(labels):
  x,y,w,h0=pos[i];state=states[i] if states else 'I'; col={'I':BLUE,'P':AMBER,'D':GRAY,'X':RED,'V':TEAL,'N':GRAY}.get(state,BLUE)
  d.add(Rect(x,y,w,h0,fillColor=WHITE,strokeColor=col,strokeWidth=1.5,strokeDashArray=[4,2] if state in ['D','N'] else None));d.add(Rect(x,y+h0-5,w,5,fillColor=col,strokeColor=None))
  lines=label.split('|');
  for j,l in enumerate(lines): text(d,x+8,y+h0-20-j*12,l,9,col)
 text(d,18,24,'I implemented/unverified  P partial  D disconnected  X unsafe  V verified  N proposed',7.5,GRAY);text(d,18,10,note[:108],7.5,GRAY);register(key,caption,d)
def build():
 s=json.loads((ROOT/'scorecard.json').read_text()); f=json.loads((ROOT/'findings.json').read_text())['findings'];subs=s['subsystems'];fts=s['features']
 bars('subsystems','Subsystem evidence score',[x['subsystem'] for x in subs],[x['raw_score'] for x in subs],100,unit='%',note='Raw subsystem evidence scores; overall safety cap = 20/100.')
 counts=s['feature_counts'];labs=['VERIFIED','PARTIAL','PLACEHOLDER','DISCONNECTED','MISSING','BLOCKED','IMPLEMENTED BUT UNVERIFIED'];bars('status','Feature-status distribution',labs,[counts.get(k,0) for k in labs],colors=[TEAL,AMBER,RED,GRAY,GRAY,RED,BLUE],note='64 scoped capabilities. Status is not code size or deployment coverage.')
 sever=collections.Counter(x['severity'] for x in f);bars('severity','Finding severity distribution',['Critical','High','Medium','Low'],[sever[k] for k in ['Critical','High','Medium','Low']],colors=[RED,AMBER,BLUE,GRAY],note='Review findings, not a count of independent exploits.')
 bars('implementation','Implementation versus verification',['Implementation criterion met','Wiring criterion met','Passing test criterion met','Runtime criterion met','Reproduction criterion met'],[sum(x[k] for x in fts) for k in ['implemented','wired','tested','executed','reproducible']],64,note='Criteria are deliberately separate; dependency inclusion is not runtime proof.')
 # Radar uses raw subsystem evidence values, all on a common 0..100 axis.
 d=Drawing(500,390);title(d,'Readiness radar — evidence, not probability');cx,cy,r=250,194,128;n=len(subs)
 for level in [20,40,60,80,100]:
  pts=[]
  for i in range(n):a=math.pi/2-2*math.pi*i/n;pts +=[cx+math.cos(a)*r*level/100,cy+math.sin(a)*r*level/100]
  d.add(Polygon(pts,fillColor=None,strokeColor=HexColor('#CFD9DF'),strokeWidth=.6))
 pts=[]
 for i,z in enumerate(subs):
  a=math.pi/2-2*math.pi*i/n;x,y=cx+math.cos(a)*r,cy+math.sin(a)*r;d.add(Line(cx,cy,x,y,strokeColor=LIGHT));text(d,cx+math.cos(a)*(r+28),cy+math.sin(a)*(r+20),z['subsystem'],8,anchor='middle');pts +=[cx+math.cos(a)*r*z['raw_score']/100,cy+math.sin(a)*r*z['raw_score']/100]
 d.add(Polygon(pts,fillColor=Color(0,.49,.51,.20),strokeColor=TEAL,strokeWidth=2));text(d,18,16,'0 at center; rings 20, 40, 60, 80, 100. Overall score is separately safety-capped.',8,GRAY);register('radar','Readiness radar: equal axes, evidence-derived raw scores',d)
 d=Drawing(500,315);title(d,'Risk heatmap — ordinal review estimates');
 for impact in range(1,6):
  for likelihood in range(1,6):
   x=70+(likelihood-1)*72;y=42+(impact-1)*44;count=sum(z['risk']==impact and z['likelihood']==likelihood for z in f);col=RED if impact*likelihood>=16 else AMBER if impact*likelihood>=9 else TEAL;d.add(Rect(x,y,68,40,fillColor=Color(col.red,col.green,col.blue,.15),strokeColor=WHITE));text(d,x+34,y+14,str(count),12,col,'middle')
 for i in range(1,6):text(d,104+(i-1)*72,28,str(i),9);text(d,52,56+(i-1)*44,str(i),9)
 text(d,220,10,'Likelihood rank (1–5); impact rank on vertical axis',8,GRAY);register('risk','Risk heatmap: finding counts by inferred likelihood and impact, not probabilities',d)
 flow('architecture','System architecture',['Wallet / SDK|P: wire encoders','RPC / pool|I: native tx path','Aura + GRANDPA|I: node wiring','FRAME runtime|P: custom pallets','Gateway / indexer|D: main exits','External chains|P: proof trust'],[(0,1),(1,2),(2,3),(3,4),(4,5),(3,5)],states=['P','I','I','P','D','P'])
 flow('transaction','Transaction lifecycle',['Client signing|P: SDK defects','Native RPC|I: submit extrinsic','Pool validation|I: SignedExtra','Execute in block|I: Executive','State commitment|I: backend','Finality / receipt|Not executed here'],states=['P','I','I','I','I','P'])
 flow('consensus','Consensus and finality',['Genesis session keys|I: Aura / GRANDPA','Aura import queue|I: slot validation','Block authoring|I: proposer','GRANDPA voter|I: default finality','Flash opt-in|X: missing membership','Finality anchors|X: unsigned hash'],[(0,1),(1,2),(2,3),(0,4),(3,5),(4,5)],states=['I','I','I','I','X','X'])
 flow('trust','Trust boundaries',['Untrusted clients|Signed / unsigned calls','Authority consensus|Known session keys','Runtime storage|Canonical balances','Governance / council|Route + upgrade rights','Relayer / external RPC|Untrusted observations','External verifier|Must prove settlement'],[(0,2),(1,2),(3,2),(4,2),(2,5)],states=['N','I','P','I','P','P'])
 flow('dependencies','Component dependencies',['node|sc-service / sc-network','runtime|FRAME / sp-api','X3 pallets|kernel / settlement','VM integrations|mini EVM / SVM / X3','proof router|vault / RLP / hashes','gateway executable|Source modules omitted'],[(0,1),(1,2),(2,3),(2,4),(5,4)],states=['I','I','P','P','P','D'])
 flow('critical-path','Critical-path completion roadmap',['0: Contain defects|C01–C03 / H01','1: Build reproducibly|H18 / H13–H15','2: Integrate execution|H07–H12','3: Private testnet|Finalized native tx','4: Adversarial closure|Proofs / partitions','5: Mainnet hardening|Keys / upgrades','6: Performance proof|Finalized load tests','7: Independent review|Release sign-off'],cols=2,states=['N']*8,note='Proposed sequence. No milestone is represented as already passed.')
 flow('timeline','7 / 30 / 60 / 90-day planning horizons',['Day 7 review|Freeze state / P0 scope','Day 30 review|Build + P0 closure','Day 60 review|Private network drills','Day 90 review|Public gate evidence'],cols=2,states=['N']*4,note='Review horizons only; dependent on staffing and acceptance gates, not delivery promises.')
 flow('startup','Node startup path',['main / run|CLI parse + chain spec','new_partial|WASM / DB / keystore','Pool + import queue|Aura + GRANDPA import','new_full|Network + RPC tasks','Authority role|Aura proposer','Consensus selection|GRANDPA or flash flag'],states=['I','I','I','I','I','P'])
 flow('state','State transition and persistence',['SignedExtra|Nonce / era / fees','Executive dispatch|Runtime state overlay','Custom VM calls|P: adapter semantics','Commit block|FRAME storage root','Finalize / index|P: gateway disconnected','Restart / restore|Unverified recovery'],states=['I','I','P','I','D','P'])
 flow('deployment','Proposed isolated testnet topology',['Operator host|Offline custody keys','4 validator processes|Distinct keys / data','Bootnode / P2P|Restricted admin access','RPC edge|TLS + request limits','Indexer + database|Private service network','Metrics + backup|Off-host authenticated copy'],states=['N']*6,note='Proposed topology for validation; no live infrastructure was contacted or deployed.')
 flow('external','External dependency map',['Polkadot SDK|stable2512 git branch','Frontier / Solana|Optional VM stacks','Rust + native libs|WASM / LLVM / protobuf','EVM chains|Header / receipt trust','Solana clusters|Validator finality trust','Bitcoin network|Best-work + output proof'],edges=[(0,1),(0,2),(1,3),(1,4),(2,5)],states=['I','I','I','P','P','P'])
 flow('attack','Attack tree: false settlement',['Goal: false settlement|or denial of settlement','Poison header|C01: signed claim','Poison finality hash|C02: unsigned anchor','Forge proof route|H01: nonempty bytes','Alter rollback data|C03: unsigned diff','Corrupt observation|H03 / H10'],[(0,1),(0,2),(0,3),(0,4),(0,5)],states=['X']*6,note='Attack hypotheses tied to findings; fund-theft end-to-end exploitation was not performed.')
 flow('privilege','Privilege map',['Signed user|Native tx / header / report','Unsigned origin|Anchors / leg receipts','Council or root|Gateway proof operations','Session authority|Aura / GRANDPA','Contract owner|Rotate verifier / mode','Custody signer|Relayer signing'],edges=[(0,2),(1,2),(3,2),(4,2),(5,2)],states=['P','X','I','I','P','P'])
 flow('keys','Key lifecycle acceptance plan',['Generate offline|OS entropy / hardware','Enroll public keys|Genesis + session binding','Sign with context|Chain / era / payload','Rotate with overlap|Session-safe activation','Revoke compromised key|Governed emergency path','Recover / audit|No seed in logs/backups'],states=['N']*6,note='Required lifecycle. Secure custody and recovery were not demonstrated in this audit.')
 # Test matrix is observed test execution status, never line coverage.
 d=Drawing(500,300);title(d,'Test evidence matrix');cols=['Present','Ran','Passed'];labels=[('RPC algorithm',[1,1,1]),('Python parsing / types',[1,1,1]),('Python emitters',[1,1,0]),('Proof-router rejection',[1,1,0]),('Full workspace',[1,0,0]),('Two-node finality',[1,0,0]),('Contracts / migrations',[1,0,0])]
 for j,k in enumerate(cols):text(d,235+j*82,250,k,9)
 for i,(lab,vs) in enumerate(labels):
  y=222-i*26;text(d,18,y,lab,9)
  for j,v in enumerate(vs):d.add(Rect(228+j*82,y-5,64,18,fillColor=TEAL if v else LIGHT,strokeColor=None));text(d,260+j*82,y,'YES' if v else 'NO',8,WHITE if v else GRAY,'middle')
 text(d,18,15,'Build attempts ran; full workspace tests did not execute. This is not line coverage.',8,GRAY);register('test-matrix','Test evidence matrix: observed execution status, not coverage percentage',d)
 bars('gates','Launch gate dashboard',['Internal devnet','Private multi-node','Public testnet','Incentivized testnet','Release candidate','Mainnet'],[0]*6,1,colors=[RED]*6,note='0 = gate not established. Requirements are in Chapter 14; no waiver of safety gates.')
 d=Drawing(500,300);title(d,'Effort versus impact — ordinal prioritization');
 for x in range(1,5):text(d,80+x*82,28,['','S','M','L','XL'][x],10);d.add(Line(80+x*82,42,80+x*82,250,strokeColor=LIGHT))
 for y in [2,3,4,5]:text(d,50,45+(y-2)*60,str(y),10);d.add(Line(70,48+(y-2)*60,445,48+(y-2)*60,strokeColor=LIGHT))
 groups=collections.defaultdict(list)
 for z in f:groups[({'S':1,'M':2,'L':3,'XL':4}[z['complexity']],z['risk'])].append(z['id'])
 for (x,y),ids in groups.items():
  xx,yy=80+x*82,48+(y-2)*60;d.add(Circle(xx,yy,7,fillColor=RED if y==5 else AMBER,strokeColor=None));text(d,xx+10,yy-3,str(len(ids))+' findings',8)
 text(d,18,9,'Complexity S/M/L/XL; impact 2–5. Group counts; no person-day or cost estimates.',8,GRAY);register('effort','Effort-versus-impact prioritization: ordinal estimates and grouped finding counts',d)
 bars('readiness','Overall readiness evidence',['Uncapped weighted score','Safety-capped score'],[s['uncapped_score'],s['readiness_score']],100,colors=[BLUE,RED],unit='/100',note='Any unresolved Critical finding caps readiness at 20; this is not a probability of safety.')
 bars('docs','Documentation versus executable evidence',['Registry feature rows','Scoped audited capabilities','Fully verified scoped behaviors','Workspace checks passing'],[15,len(fts),sum(z['status']=='VERIFIED' for z in fts),0],maxv=64,note='Different scopes shown as counts; registry percentages are not converted into runtime credit.')
 # Dependency matrix between stop-the-line fixes and later work.
 flow('dependency-heatmap','Critical dependency coverage',['C01 header authenticity|Blocks external settlement','C02 finality anchors|Blocks PoAE finalization','C03 rollback provenance|Blocks atomicity claims','H18 build proof|Blocks node execution','H13 CI propagation|Blocks trustworthy gates','H14 artifact binding|Blocks release provenance'],cols=2,states=['X','X','X','P','P','P'],note='Qualitative dependency matrix. All six are prerequisites, not measured risk scores.')
 return DRAWS
if __name__=='__main__':
 build();(ROOT/'assets').mkdir(exist_ok=True)
 for k,d in DRAWS.items():renderSVG.drawToFile(d,str(ROOT/'assets'/f'{k}.svg'));renderPDF.drawToFile(d,str(ROOT/'assets'/f'{k}.pdf'))
 (ROOT/'figure-register.json').write_text(json.dumps([{'number':i+1,'id':k,'caption':CAPTIONS[k],'svg':f'assets/{k}.svg','pdf':f'assets/{k}.pdf'} for i,k in enumerate(DRAWS)],indent=2))
 print(len(DRAWS),'vector figures generated')
