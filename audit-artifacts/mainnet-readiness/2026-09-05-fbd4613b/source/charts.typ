// Hand-rolled horizontal bar charts using cetz core drawing primitives only
// (cetz.plot is not used: the cached cetz 0.3.1 plotting submodule is a
// stub that panics — see README.md for detail). Horizontal bar charts are
// used throughout instead of pie or radar charts: bars never exaggerate
// scale relationships the way radial encodings can, and every chart here
// renders directly from data.typ / findings.json / feature-matrix.csv.

#import "@preview/cetz:0.3.1": canvas, draw
#import "style.typ": *

#let hbar-chart(
  data,
  max-value: none,
  unit: "",
  bar-width: 11,
  bar-height: 0.62,
  gap: 0.22,
  label-width: 5.6,
  gridlines: 4,
) = {
  let mx = if max-value == none { data.map(d => d.value).fold(0, calc.max) } else { max-value }
  canvas(length: 1cm, {
    import draw: *
    set-style(stroke: (thickness: 0.4pt))

    // gridlines + axis labels
    for i in range(0, gridlines + 1) {
      let gx = label-width + (i / gridlines) * bar-width
      let gv = calc.round(mx * i / gridlines)
      line((gx, 0.35), (gx, -(data.len() * (bar-height + gap)) + gap*0.3),
        stroke: (paint: rgb("#dcdce3"), thickness: 0.35pt))
      content((gx, 0.55), text(size: 7pt, fill: rgb("#7a7a8a"))[#gv#unit], anchor: "center")
    }

    for (i, d) in data.enumerate() {
      let y = -(i * (bar-height + gap))
      let bar-len = if mx == 0 { 0 } else { (d.value / mx) * bar-width }
      content((label-width - 0.2, y - bar-height/2), text(size: 8.3pt, fill: rgb("#1a1a2e"))[#d.label], anchor: "east")
      rect((label-width, y - bar-height), (label-width + bar-width, y),
        fill: rgb("#f0f0f4"), stroke: none)
      if bar-len > 0 {
        rect((label-width, y - bar-height), (label-width + bar-len, y),
          fill: d.color, stroke: none)
      }
      content((label-width + bar-len + 0.18, y - bar-height/2),
        text(size: 8pt, weight: "bold", fill: rgb("#333340"))[#d.value#unit], anchor: "west")
    }
  })
}

#let severity-bar-chart(sev-counts) = {
  let colors = (
    Critical: rgb("#8c1c13"),
    High: rgb("#a8460a"),
    Medium: rgb("#8a6d00"),
    Low: rgb("#2e6b32"),
    Informational: rgb("#2d5a8a"),
  )
  let data = sev-counts.map(r => (label: r.sev, value: r.count, color: colors.at(r.sev)))
  hbar-chart(data, max-value: calc.max(..sev-counts.map(r => r.count)) + 2, label-width: 4.4, bar-width: 9)
}

#let score-band-color(score) = {
  if score < 40 { rgb("#8c1c13") }
  else if score < 70 { rgb("#a8460a") }
  else { rgb("#2e6b32") }
}

#let subsystem-score-chart(rows) = {
  let sorted = rows.sorted(key: r => -r.score)
  let data = sorted.map(r => (label: r.name, value: r.score, color: score-band-color(r.score)))
  hbar-chart(data, max-value: 100, unit: "", label-width: 6.4, bar-width: 8.6, gridlines: 4, bar-height: 0.5, gap: 0.18)
}

#let feature-status-chart(rows) = {
  let colors = (
    "Verified": rgb("#2e6b32"),
    "Partial": rgb("#8a6d00"),
    "Implemented, unverified": rgb("#2d5a8a"),
    "Placeholder": rgb("#8c1c13"),
    "Disconnected": rgb("#a8460a"),
    "Blocked": rgb("#6a6a78"),
    "Informational": rgb("#7a7a8a"),
  )
  let data = rows.map(r => (label: r.status, value: r.count, color: colors.at(r.status)))
  hbar-chart(data, max-value: calc.max(..rows.map(r => r.count)) + 3, label-width: 5.8, bar-width: 8.6)
}
