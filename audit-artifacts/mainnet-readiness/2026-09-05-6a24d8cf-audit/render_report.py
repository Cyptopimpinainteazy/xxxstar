"""Render the frozen report-source.json to a paginated, bookmarked PDF."""
from pathlib import Path
import json,re,html,sys,collections,copy
from reportlab.platypus import BaseDocTemplate,PageTemplate,Frame,Paragraph,Spacer,PageBreak,Table,TableStyle,KeepTogether,Flowable
from reportlab.platypus.tableofcontents import TableOfContents
from reportlab.lib.styles import getSampleStyleSheet,ParagraphStyle
from reportlab.lib.colors import HexColor,white
from reportlab.lib.enums import TA_LEFT
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib.pagesizes import A4
import visuals
R=Path(__file__).resolve().parent
D=json.loads((R/'report-source.json').read_text());S=json.loads((R/'scorecard.json').read_text());F=json.loads((R/'findings.json').read_text())['findings'];FIG=json.loads((R/'figure-register.json').read_text());visuals.build()
for name,file in [('Body','DejaVuSans.ttf'),('Bold','DejaVuSans-Bold.ttf'),('Mono','DejaVuSansMono.ttf')]:pdfmetrics.registerFont(TTFont(name,'/usr/share/fonts/truetype/dejavu/'+file))
pdfmetrics.registerFontFamily('Body',normal='Body',bold='Bold',italic='Body',boldItalic='Bold')
NAVY=HexColor('#142B40');TEAL=HexColor('#007C83');RED=HexColor('#B22F3D');GRAY=HexColor('#536571');LIGHT=HexColor('#EEF3F6');CREAM=HexColor('#F7F8F5')
W,H=A4; M=48;CW=W-2*M
styles={
 'body':ParagraphStyle('BodyText',fontName='Body',fontSize=9.6,leading=14,textColor=NAVY,spaceAfter=8,splitLongWords=True),
 'compact':ParagraphStyle('Compact',fontName='Body',fontSize=9.2,leading=13,textColor=NAVY,spaceAfter=6,splitLongWords=True),
 'small':ParagraphStyle('Small',fontName='Body',fontSize=8,leading=11.5,textColor=GRAY,spaceAfter=6,splitLongWords=True),
 'chapter':ParagraphStyle('ChapterTitle',fontName='Bold',fontSize=27,leading=35,textColor=NAVY,spaceAfter=22,keepWithNext=True,splitLongWords=True),
 'h2':ParagraphStyle('H2',fontName='Bold',fontSize=13,leading=18,textColor=NAVY,spaceBefore=14,spaceAfter=8,keepWithNext=True),
 'caption':ParagraphStyle('Caption',fontName='Bold',fontSize=8.6,leading=12,textColor=TEAL,spaceBefore=7,spaceAfter=10,keepWithNext=False),
 'cell':ParagraphStyle('Cell',fontName='Body',fontSize=8.1,leading=11.4,textColor=NAVY,splitLongWords=True),
 'head':ParagraphStyle('TableHead',fontName='Bold',fontSize=8.2,leading=11.4,textColor=white,splitLongWords=True),
 'code':ParagraphStyle('Code',fontName='Mono',fontSize=7.8,leading=11.5,textColor=NAVY,backColor=LIGHT,borderPadding=10,spaceBefore=5,spaceAfter=12,splitLongWords=True),
 'callout':ParagraphStyle('Callout',fontName='Bold',fontSize=10,leading=15,textColor=RED,backColor=HexColor('#FFF0EE'),borderPadding=12,spaceBefore=8,spaceAfter=16,splitLongWords=True),
 'kicker':ParagraphStyle('Kicker',fontName='Bold',fontSize=9,leading=13,textColor=TEAL,spaceAfter=15),
}
def safe(s):return html.escape(str(s)).replace('\n','<br/>')
def para(s,style='body'):return Paragraph(safe(s),styles[style])
class Cover(Flowable):
 def __init__(self):Flowable.__init__(self);self.width=CW;self.height=H-110
 def draw(self):
  c=self.canv;c.setFillColor(TEAL);c.rect(0,self.height-22,68,5,fill=1,stroke=0)
  def t(y,s,size=12,font='Body',color=NAVY):c.setFont(font,size);c.setFillColor(color);c.drawString(0,y,s)
  t(self.height-52,'X3 / ENGINEERING FIELD MANUAL',10,'Bold',TEAL)
  t(self.height-124,'X3 ATOMIC STAR',29,'Bold')
  t(self.height-181,'THE ROAD',43,'Bold')
  t(self.height-237,'TO MAINNET',43,'Bold')
  pp=para(D['subtitle'],'h2');pp.wrap(CW-20,100);pp.drawOn(c,0,self.height-325)
  c.setStrokeColor(HexColor('#CCD8DE'));c.line(0,self.height-360,CW,self.height-360)
  t(self.height-401,'AUDITED EVIDENCE READINESS',9,'Bold',GRAY)
  t(self.height-468,f'{S["readiness_score"]:g}',55,'Bold',RED);c.setFont('Body',18);c.drawString(91,self.height-466,'/ 100')
  c.setFont('Bold',11);c.setFillColor(RED);c.drawString(218,self.height-425,'PUBLIC TESTNET: NO-GO');c.drawString(218,self.height-451,'MAINNET: NO-GO')
  t(self.height-508,'Three Critical findings remain open.',12,'Bold',RED)
  t(105,'05 SEPTEMBER 2026  /  MASTER + EXISTING EDITS',10,'Bold')
  t(82,'Codex / AI-assisted analysis',10)
  t(60,'Base commit: '+D['provenance']['commit'],7.6,'Mono')
  t(34,'Read-only protocol audit • Evidence package • Completion blueprint',8.5)
class Rule(Flowable):
 def __init__(self):Flowable.__init__(self);self.width=CW;self.height=12
 def draw(self):self.canv.setStrokeColor(TEAL);self.canv.setLineWidth(2);self.canv.line(0,6,CW,6)
class Doc(BaseDocTemplate):
 def __init__(self,path):
  super().__init__(str(path),pagesize=A4,rightMargin=M,leftMargin=M,topMargin=55,bottomMargin=48,title=D['title'],author='Codex / AI-assisted analysis',subject=D['subtitle'],pageCompression=1)
  self.addPageTemplates(PageTemplate(id='main',frames=[Frame(M,48,CW,H-103,id='normal',leftPadding=0,rightPadding=0,topPadding=0,bottomPadding=0)],onPage=self.decorate))
  self.outline=[]
 def decorate(self,c,doc):
  c.saveState()
  if doc.page==1:c.setFillColor(CREAM);c.rect(0,0,W,H,fill=1,stroke=0)
  else:
   c.setFont('Bold',7);c.setFillColor(TEAL);c.drawString(M,H-28,'X3  /  THE ROAD TO MAINNET');c.setFont('Body',7);c.setFillColor(GRAY);c.drawRightString(W-M,H-28,'EVIDENCE-BASED ENGINEERING REVIEW')
  c.setStrokeColor(HexColor('#D5DFE4'));c.setLineWidth(.4);c.line(M,35,W-M,35);c.setFont('Body',6.7);c.setFillColor(GRAY);c.drawString(M,23,'2026-09-05  •  6a24d8cf + audited working-tree changes');c.drawRightString(W-M,23,str(doc.page));c.restoreState()
 def afterFlowable(self,f):
  if isinstance(f,Paragraph) and f.style.name=='ChapterTitle':
   text=f.getPlainText();key='sec-'+re.sub('[^a-zA-Z0-9]','-',text);self.canv.bookmarkPage(key);self.canv.addOutlineEntry(text,key,0,False);self.notify('TOCEntry',(0,text,self.page,key));self.outline.append({'title':text,'page':self.page,'key':key})
def tab(block,num):
 n=len(block['headers']);widths={2:[CW*.31,CW*.69],3:[CW*.25,CW*.30,CW*.45],4:[CW*.17,CW*.23,CW*.30,CW*.30]}.get(n,[CW/n]*n)
 data=[[para(x,'head') for x in block['headers']]]+[[para(x,'cell') for x in row] for row in block['rows']]
 t=Table(data,colWidths=widths,repeatRows=1,hAlign='LEFT',splitByRow=1)
 t.setStyle(TableStyle([('BACKGROUND',(0,0),(-1,0),NAVY),('ROWBACKGROUNDS',(0,1),(-1,-1),[white,HexColor('#F4F7F9')]),('VALIGN',(0,0),(-1,-1),'TOP'),('LEFTPADDING',(0,0),(-1,-1),7),('RIGHTPADDING',(0,0),(-1,-1),7),('TOPPADDING',(0,0),(-1,-1),6),('BOTTOMPADDING',(0,0),(-1,-1),6),('LINEBELOW',(0,0),(-1,0),.6,TEAL),('LINEBELOW',(0,1),(-1,-1),.25,HexColor('#DCE5EA'))]))
 caption=para('Table '+str(num)+'  '+block['caption'],'caption');caption.keepWithNext=True
 return [caption,t,Spacer(1,12)]
def render():
 story=[Cover(),PageBreak(),para('Scope, provenance and how to read','chapter'),para(D['classification'],'kicker')]
 story += [para('Intended audience: protocol engineers, security reviewers, validators, testnet operators, grant/sponsor reviewers and future contributors. The base commit is '+D['provenance']['commit']+' on master, with extensive pre-existing tracked and untracked edits. The audited source is not a clean checkout of that commit.'),para('Safety and scope: protocol code was inspected without modification. Safe local builds/tests used an isolated source copy, offline Cargo and disposable outputs. No deployments, public-network signing, transactions, live-fund interaction or production credentials were used. Audit artifacts are the only intended repository changes.'),para('The review gives an evidence-based readiness decision and completion plan. It is not independent security certification, a formal cryptographic proof, a line-by-line review of every vendored or first-party file, or a claim that blocked experiments passed. Execution, static inspection, inference and documentation claims are distinguished.'),para('Reading order: start with Chapter 1, then Chapter 5 for actionable defects. Engineers should use Chapters 7–12 and the finding/feature JSON/CSV files. Operators should use Chapters 11 and 14. Sponsors can use executive-summary.md and Chapter 13. Every numbered figure is a reusable vector asset; all measurements and scores derive from shipped data.'),para('Evidence references such as C01, H18, FT01 and file.rs:123 lead to the finding register, feature scorecard and exact audited source. PDF page numbers include front matter. Checksums and final page count are recorded externally after rendering, avoiding circular self-hashes.'),PageBreak(),para('Contents','chapter')]
 toc=TableOfContents();toc.levelStyles=[ParagraphStyle('TOC',fontName='Body',fontSize=9.5,leading=13.8,textColor=NAVY,leftIndent=0,firstLineIndent=0,spaceBefore=3)];story +=[toc,PageBreak(),para('Figures and tables','chapter')]
 for fig in FIG:story.append(para('Figure '+str(fig['number'])+' — '+fig['caption']+'  [assets/'+fig['id']+'.svg]','small'))
 tablelist=[];tn=0
 for sec in D['sections']:
  for block in sec['blocks']:
   if block['type']=='table':tn+=1;tablelist.append((tn,block['caption']))
 story +=[PageBreak(),para('List of tables','h2')]+[para('Table '+str(n)+' — '+cap,'small') for n,cap in tablelist]
 tn=0
 for ix,sec in enumerate(D['sections']):
  story +=[PageBreak(),Spacer(1,38),para(sec['kicker'],'kicker'),para(sec['title'],'chapter'),Rule()]
  for j,block in enumerate(sec['blocks']):
   typ=block['type']
   if ix==13 and j==len(sec['blocks'])-4:
    story.append(KeepTogether([para(b['text'],'h2' if b['type']=='h2' else 'body') for b in sec['blocks'][-4:]]));continue
   if ix==13 and j>len(sec['blocks'])-4:continue
   if typ=='p':story.append(para(block['text'],'small' if (ix in [5,17] and j==len(sec['blocks'])-1) or (ix==16 and j>=len(sec['blocks'])-2) else 'compact' if ix==13 and j>=len(sec['blocks'])-3 else 'body'))
   elif typ=='callout':story.append(para(block['text'],'callout'))
   elif typ=='h2':story.append(para(block['text'],'h2'))
   elif typ=='code':story.append(para(block['text'],'code'))
   elif typ=='pagebreak':story.append(PageBreak())
   elif typ=='table':tn+=1;story.extend(tab(block,tn))
   elif typ=='figure':
    d=visuals.DRAWS[block['id']];scale=min(CW/d.width,570/d.height,1);dd=copy.deepcopy(d);dd.scale(scale,scale);dd.width=d.width*scale;dd.height=d.height*scale
    intro=para('Figure '+str(block['number'])+' shows '+block['caption'][0].lower()+block['caption'][1:]+'.','small')
    story.append(KeepTogether([intro,dd,para('Figure '+str(block['number'])+'  '+block['caption'],'caption')]))
   if j==0 and ix<14:
    story +=[Spacer(1,35),para('FIELD MANUAL  /  '+str(ix+1).zfill(2),'kicker'),para('Base commit '+D['provenance']['commit']+'\nAudited 2026-09-05 with pre-existing working-tree changes.','small'),PageBreak()]
 doc=Doc(R/'X3-ROAD-TO-MAINNET.pdf');doc.multiBuild(story,maxPasses=4)
 (R/'pdf-navigation.json').write_text(json.dumps({'page_count':doc.page,'chapter_pages':doc.outline[-22:],'tables':[{'number':n,'caption':c} for n,c in tablelist]},indent=2));print('PDF pages',doc.page,'tables',len(tablelist),'figures',len(FIG))
if __name__=='__main__':render()
