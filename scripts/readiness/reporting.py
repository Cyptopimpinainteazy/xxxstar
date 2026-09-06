"""Render one evaluated evidence snapshot as HTML, SVG, Markdown, CSV and PDF."""
from collections import Counter
import csv
import html
import json
from pathlib import Path

NAVY = '#142b40'
TEAL = '#007c83'
RED = '#b22f3d'


def escape(value):
    return html.escape(str(value), quote=True)


def bars(title, rows, maximum=100, unit='', color=TEAL):
    height = 75 + len(rows) * 34
    pieces = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 {height}" role="img" aria-label="{escape(title)}">',
              f'<rect width="640" height="{height}" fill="white"/><text x="16" y="26" font-family="sans-serif" font-size="18" fill="{NAVY}">{escape(title)}</text>']
    for index, (label, value) in enumerate(rows):
        y = 52 + index * 34
        pieces += [f'<text x="16" y="{y+15}" font-family="sans-serif" font-size="12" fill="{NAVY}">{escape(label)}</text>',
                   f'<rect x="270" y="{y}" width="285" height="20" fill="#edf3f6"/>',
                   f'<rect x="270" y="{y}" width="{285*value/max(maximum,1):.3f}" height="20" fill="{color}"/>',
                   f'<text x="565" y="{y+15}" font-family="sans-serif" font-size="12" fill="{NAVY}">{value:g}{escape(unit)}</text>']
    return ''.join(pieces) + '</svg>'


def data_tables(report):
    return [
        ('Tasks and finding closure', ['ID / severity', 'Task', 'Current status', 'Acceptance requirement'],
         [[t['id'] + ' / ' + t['severity'], t['description'], t['status'], t.get('acceptance', '')] for t in report['tasks']]),
        ('Verification runs', ['Check', 'Command', 'Result', 'Evidence'],
         [[c['id'], ' '.join(c['argv']), c['status'] + ': ' + c['reason'],
           (c['receipt']['id'] + ' / exit ' + str(c['receipt']['exit_code'])) if c['receipt'] else 'Not run'] for c in report['checks']]),
        ('Feature evidence', ['Feature', 'Score / status', 'Criterion provenance'],
         [[f['id'] + ' ' + f['feature'], str(f['completion_percent']) + '% / ' + f['status'],
           '; '.join(k + ': ' + v for k, v in f['criterion_sources'].items())] for f in report['features']]),
        ('Closure and criterion reviews', ['Target / criterion', 'Reviewer / date', 'Rationale'],
         [[r['target'] + ' / ' + r['criterion'], r['reviewer'] + ' / ' + r['at'], r['note']] for r in report['reviews']])]


def landing_html():
    return r'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>X3 live readiness</title>
<style>body{margin:0;font:15px system-ui;background:#eef3f6;color:#142b40}header{padding:12px 20px;border-bottom:1px solid #bbcbd5}iframe{width:100%;height:calc(100vh - 68px);border:0}#status{font-weight:600}</style>
<header><strong>X3 / LIVE READINESS</strong> · <span id="status" role="status">Loading latest snapshot…</span></header><iframe id="report" title="Current X3 readiness report"></iframe>
<script>
let last='';async function update(){try{const r=await fetch('current.json',{cache:'no-store'});if(!r.ok)throw Error('HTTP '+r.status);const p=await r.json();if(!/^snapshots\/[A-Za-z0-9-]+$/.test(p.snapshot))throw Error('Invalid snapshot path');if(last!==p.snapshot){document.getElementById('report').src=p.snapshot+'/index.html';last=p.snapshot;}document.getElementById('status').textContent='Snapshot '+p.updated_at+' · checks run only on request';}catch(e){document.getElementById('status').textContent='Refresh unavailable: '+e.message+'. Serve this directory over localhost HTTP; an already displayed snapshot may be stale.';}}update();setInterval(update,10000);
</script></html>'''


def render(report, destination, pdf=True):
    task_counts = Counter(t['status'] for t in report['tasks'])
    check_counts = Counter(c['status'] for c in report['checks'])
    figures = [
        ('readiness', 'Readiness evidence', [('Current', report['readiness_score']), ('Before safety cap', report['uncapped_score'])], 100, '/100'),
        ('subsystems', 'Readiness by subsystem', [(s['subsystem'], s['score']) for s in report['subsystems']], 100, '%'),
        ('findings', 'Open findings', list(report['open_findings'].items()), max(report['open_findings'].values(), default=1), ''),
        ('tasks', 'Task status', [(k, task_counts[k]) for k in ('planned', 'in_progress', 'awaiting_verification', 'completed')], report['task_count'], ''),
        ('checks', 'Verification status', [(k, check_counts[k]) for k in ('not_run', 'passed', 'failed', 'stale', 'invalid')], len(report['checks']), '')]
    assets = destination / 'assets'; assets.mkdir()
    for name, title, rows, maximum, unit in figures:
        (assets / (name + '.svg')).write_text(bars(title, rows, maximum, unit, RED if name == 'findings' else TEAL))
    baseline = report.get('baseline')
    note = ('Historical audit credit remains eligible on unchanged source.' if report['baseline_eligible'] else
            'Historical audit credit is absent or stale. Current credits require fresh evidence and reviews.')
    lead = f"{report['readiness_score']:g}/100 evidence readiness. {report['completed_tasks']}/{report['task_count']} tasks completed. Release decision: {report['release_decision']}."
    caveat = 'A score of 100 never grants launch approval. Closing the tracked findings changes the automated decision to NOT ASSESSED; independent launch-gate approval is still required.'
    sections = data_tables(report)
    md = ['# X3: Live Readiness Report', '', report['generated_at'], '', lead, '', note, '', report['score_formula'], '', caveat,
          '', 'Source fingerprint: `' + report['source']['fingerprint'] + '`', '', report['review_trust']]
    parts = [f'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>X3 live readiness</title><style>body{{font:16px/1.6 system-ui;margin:0;background:#f3f6f8;color:{NAVY}}}main{{max-width:1200px;margin:auto;padding:32px}}h1{{font-size:38px;margin:0}}h2{{margin-top:36px}}.eyebrow{{color:{TEAL};font-weight:700;letter-spacing:.12em}}.metrics{{display:flex;gap:24px;flex-wrap:wrap;margin:24px 0}}.metric{{background:white;padding:20px;border-top:4px solid {TEAL};flex:1;min-width:170px}}.number{{font-size:38px;font-weight:750}}.warning{{padding:18px;background:#fff0ed;border-left:4px solid {RED}}}.charts{{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:20px}}figure{{margin:0;background:white;padding:12px}}figure img{{width:100%;height:auto}}figcaption{{font-size:13px}}table{{width:100%;border-collapse:collapse;background:white;font-size:14px}}th{{background:{NAVY};color:white;text-align:left}}td,th{{padding:10px;vertical-align:top;border-bottom:1px solid #dce4e9;overflow-wrap:anywhere}}.scroll{{overflow:auto}}code{{overflow-wrap:anywhere}}a{{color:#005f89}}small{{color:#465c6a}}@media(max-width:600px){{main{{padding:16px}}h1{{font-size:28px}}}}</style><main>',
             '<div class="eyebrow">X3 / EVIDENCE OPERATIONS</div><h1>The road to mainnet</h1>',
             f'<p>Live evidence snapshot · {escape(report["generated_at"])}</p>',
             f'<div class="metrics"><div class="metric"><div class="number">{report["readiness_score"]:g}/100</div>Evidence readiness</div><div class="metric"><div class="number">{report["completed_tasks"]}/{report["task_count"]}</div>Verified task completions</div><div class="metric"><div class="number">{escape(report["release_decision"])}</div>Public testnet / mainnet</div></div>',
             f'<p class="warning">{escape(note)} {escape(caveat)}</p>',
             '<p><a href="summary.json">Data</a> · <a href="features.csv">Feature CSV</a> · <a href="source.md">Readable source</a> · <a href="manifest.json">Checksums</a>' + (' · <a href="X3-LIVE-READINESS.pdf">Download PDF</a>' if pdf else '') + '</p>',
             '<h2>Current measurements</h2><div class="charts">']
    for number, (name, title, _, _, _) in enumerate(figures, 1):
        parts.append(f'<figure><img src="assets/{name}.svg" alt="{escape(title)}"><figcaption>Figure {number}: {escape(title)}. Calculated from this snapshot.</figcaption></figure>')
        md += ['', f'![{title}](assets/{name}.svg)']
    parts += ['</div>', '<h2>How scores change</h2><p>' + escape(report['score_formula']) + '</p>',
              '<p>Task state alone earns no readiness credit. Check results must match the current source and command configuration, retain valid log hashes, and be within the evidence age limit. Each criterion needs a named review. Failed reruns, edited acceptance requirements and changed dependencies invalidate affected reviews.</p>']
    for title, headers, rows in sections:
        parts += ['<h2>' + escape(title) + '</h2><div class="scroll"><table><thead><tr>' + ''.join('<th>' + escape(x) + '</th>' for x in headers) + '</tr></thead><tbody>']
        md += ['', '## ' + title, '', '| ' + ' | '.join(headers) + ' |', '|' + '---|' * len(headers)]
        for row in rows:
            parts.append('<tr>' + ''.join('<td>' + escape(x) + '</td>' for x in row) + '</tr>')
            md.append('| ' + ' | '.join(str(x).replace('|', '/').replace('\n', ' ') for x in row) + ' |')
        parts.append('</tbody></table></div>')
    for check in report['checks']:
        if check['receipt']:
            receipt = check['receipt']
            parts.append('<p>Evidence for ' + escape(check['id']) + ': <a href="../../' + escape(receipt['log']) + '">command log</a> · <a href="../../evidence/' + escape(receipt['id']) + '.json">receipt JSON</a></p>')
    parts += ['<h2>Provenance and review limits</h2><p>' + escape(report['review_trust']) + '</p>',
              '<p>Source commit: <code>' + escape(report['source']['commit'] or 'Uncommitted repository') + '</code><br>Source fingerprint: <code>' + report['source']['fingerprint'] + '</code></p>',
              '<p>' + escape(report['source']['scope']) + '</p>']
    if baseline:
        parts.append('<p>Historical baseline: ' + escape(baseline['commit']) + '; historical readiness ' + str(baseline['readiness_score']) + '/100. Historical findings and architecture remain in the original audit; this report is the current completion and evidence supplement.</p>')
    parts.append('</main></html>')
    (destination / 'index.html').write_text(''.join(parts))
    (destination / 'source.md').write_text('\n'.join(md) + '\n')
    with (destination / 'features.csv').open('w', newline='') as handle:
        writer = csv.writer(handle); writer.writerow(['id', 'feature', 'subsystem', 'status', 'completion_percent', 'implemented', 'wired', 'tested', 'executed', 'reproducible'])
        for feature in report['features']:
            writer.writerow([feature.get(k, '') for k in ['id', 'feature', 'subsystem', 'status', 'completion_percent', 'implemented', 'wired', 'tested', 'executed', 'reproducible']])
    if pdf:
        render_pdf(report, destination, figures, sections, lead, note, caveat)


def render_pdf(report, destination, figures, sections, lead, note, caveat):
    # Deliberately required: a missing renderer fails publication instead of silently dropping the PDF.
    from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, PageBreak, KeepTogether
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.lib.colors import HexColor, white
    from reportlab.lib.pagesizes import A4
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.graphics.shapes import Drawing, Rect, String
    for name, filename in [('LiveBody', 'DejaVuSans.ttf'), ('LiveBold', 'DejaVuSans-Bold.ttf')]:
        if name not in pdfmetrics.getRegisteredFontNames():
            pdfmetrics.registerFont(TTFont(name, '/usr/share/fonts/truetype/dejavu/' + filename))
    styles = {'body': ParagraphStyle('body', fontName='LiveBody', fontSize=9, leading=13, spaceAfter=8, textColor=HexColor(NAVY)),
              'h1': ParagraphStyle('h1', fontName='LiveBold', fontSize=29, leading=36, spaceAfter=20),
              'h2': ParagraphStyle('h2', fontName='LiveBold', fontSize=15, leading=20, spaceAfter=12, keepWithNext=True),
              'head': ParagraphStyle('head', fontName='LiveBold', fontSize=8, leading=11, textColor=white),
              'cell': ParagraphStyle('cell', fontName='LiveBody', fontSize=8, leading=11)}
    def p(text, style='body'):
        return Paragraph(escape(text).replace('\n', '<br/>'), styles[style])
    story = [Spacer(1, 40), p('X3 / LIVE READINESS', 'h2'), p('THE ROAD TO MAINNET', 'h1'), p(lead, 'h2'), p(report['generated_at']), p(note), p(caveat), p(report['score_formula']), p('Source: ' + report['source']['fingerprint']), p(report['review_trust']), PageBreak()]
    for number, (_, title, rows, maximum, unit) in enumerate(figures, 1):
        drawing = Drawing(490, 40 + len(rows) * 24)
        for i, (label, value) in enumerate(rows):
            y = drawing.height - 28 - i * 24
            drawing.add(String(0, y, label, fontName='LiveBody', fontSize=8, fillColor=HexColor(NAVY)))
            drawing.add(Rect(200, y-2, 220, 13, fillColor=HexColor('#edf3f6'), strokeColor=None))
            drawing.add(Rect(200, y-2, 220*value/max(maximum, 1), 13, fillColor=HexColor(TEAL), strokeColor=None))
            drawing.add(String(428, y, f'{value:g}{unit}', fontName='LiveBody', fontSize=8))
        story.append(KeepTogether([p(f'Figure {number} · {title}', 'h2'), drawing, Spacer(1, 15)]))
    for title, headers, rows in sections:
        story += [PageBreak(), p(title, 'h1')]
        if not rows:
            story.append(p('No records yet.')); continue
        widths = [90, 120, 90, 199] if len(headers) == 4 else [150, 110, 239]
        data = [[p(x, 'head') for x in headers]] + [[p(x, 'cell') for x in row] for row in rows]
        table = Table(data, colWidths=widths, repeatRows=1, hAlign='LEFT')
        table.setStyle(TableStyle([('BACKGROUND', (0,0), (-1,0), HexColor(NAVY)), ('ROWBACKGROUNDS', (0,1), (-1,-1), [white, HexColor('#f0f5f7')]), ('VALIGN', (0,0), (-1,-1), 'TOP'), ('TOPPADDING', (0,0), (-1,-1), 6), ('BOTTOMPADDING', (0,0), (-1,-1), 6)]))
        story.append(table)
    def footer(canvas, doc):
        canvas.setFont('LiveBody', 7); canvas.setFillColor(HexColor(NAVY))
        canvas.drawString(48, 25, 'X3 live evidence · ' + report['generated_at'][:10] + ' · ' + report['source']['fingerprint'][:12])
        canvas.drawRightString(A4[0]-48, 25, str(doc.page))
    doc = SimpleDocTemplate(str(destination / 'X3-LIVE-READINESS.pdf'), pagesize=A4, leftMargin=48, rightMargin=48, topMargin=48, bottomMargin=48, title='X3: Live Readiness Report', author='X3 local evidence workflow / operator-reviewed records')
    doc.build(story, onFirstPage=footer, onLaterPages=footer)
