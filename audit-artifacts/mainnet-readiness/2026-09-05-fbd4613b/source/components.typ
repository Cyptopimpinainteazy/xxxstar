#import "style.typ": *

// ---- severity / status color lookup -----------------------------------

#let sev-color(sev) = {
  let s = lower(sev)
  if s == "critical" { (fg: c-critical, bg: c-critical-bg) }
  else if s == "high" { (fg: c-high, bg: c-high-bg) }
  else if s == "medium" { (fg: c-medium, bg: c-medium-bg) }
  else if s == "low" { (fg: c-low, bg: c-low-bg) }
  else { (fg: c-info, bg: c-info-bg) }
}

#let severity-badge(sev) = {
  let col = sev-color(sev)
  box(fill: col.bg, stroke: 0.6pt + col.fg, radius: 2pt, inset: (x: 5pt, y: 2pt))[
    #text(font: title-font, size: 7.6pt, weight: "bold", fill: col.fg, tracking: 0.06em)[#upper(sev)]
  ]
}

#let status-word-color(status) = {
  let s = upper(status)
  if s.contains("VERIFIED") and not s.contains("UNVERIFIED") { c-low }
  else if s.contains("PARTIAL") { c-medium }
  else if s.contains("PLACEHOLDER") { c-critical }
  else if s.contains("DISCONNECTED") { c-high }
  else if s.contains("BLOCKED") { c-muted }
  else if s.contains("MISSING") { c-critical }
  else if s.contains("UNVERIFIED") { c-info }
  else { c-muted }
}

#let status-badge(status) = {
  let col = status-word-color(status)
  box(fill: white, stroke: 0.7pt + col, radius: 2pt, inset: (x: 5pt, y: 2pt))[
    #text(font: title-font, size: 7.4pt, weight: "bold", fill: col)[#upper(status)]
  ]
}

// ---- evidence quality tag ----------------------------------------------

#let evidence-label(kind) = {
  let k = lower(kind)
  let (fg, label) = if k.contains("execution") {
    (c-low, "CONFIRMED BY EXECUTION")
  } else if k.contains("static") {
    (c-info, "CONFIRMED BY STATIC INSPECTION")
  } else if k.contains("infer") {
    (c-medium, "INFERRED")
  } else if k.contains("document") or k.contains("claim") {
    (c-high, "CLAIMED BY DOCUMENTATION")
  } else if k.contains("not verified") {
    (c-muted, "NOT VERIFIED")
  } else if k.contains("block") {
    (c-critical, "BLOCKED")
  } else {
    (c-muted, upper(kind))
  }
  text(font: title-font, size: 7.6pt, weight: "bold", fill: fg, tracking: 0.04em)[◆ #label]
}

// ---- callout box ---------------------------------------------------------

#let callout(kind: "info", title: none, body) = {
  let col = if kind == "critical" { c-critical }
    else if kind == "warning" { c-high }
    else if kind == "scope-note" { c-brand }
    else { c-info }
  block(
    width: 100%,
    fill: c-bg-panel,
    stroke: (left: 2.6pt + col),
    inset: (left: 10pt, right: 10pt, top: 8pt, bottom: 8pt),
    radius: 2pt,
    breakable: true,
  )[
    #if title != none [
      #text(font: title-font, size: 9.5pt, weight: "bold", fill: col)[#title]
      #v(0.25em)
    ]
    #set text(size: 9.3pt)
    #body
  ]
}

#let pull-quote(body, source: none) = {
  block(width: 100%, inset: (left: 14pt, top: 6pt, bottom: 6pt))[
    #set text(font: body-font, size: 13pt, style: "italic", fill: c-brand)
    #block(stroke: (left: 1.6pt + c-brand), inset: (left: 10pt))[
      "#body"
      #if source != none [
        #v(0.3em)
        #text(font: title-font, size: 8pt, style: "normal", fill: c-muted)[— #source]
      ]
    ]
  ]
}

// ---- finding card ---------------------------------------------------------

#let field-line(label, value) = {
  if value == none or value == "" { return }
  block(width: 100%, breakable: true)[
    #text(font: title-font, size: 8.2pt, weight: "bold", fill: c-muted)[#upper(label): ]
    #text(size: 9.2pt)[#value]
  ]
  v(0.28em)
}

#let finding-card(f) = {
  block(
    width: 100%,
    fill: white,
    stroke: 0.7pt + c-rule,
    radius: 3pt,
    inset: 10pt,
    breakable: true,
    above: 0.8em,
    below: 0.8em,
  )[
    #grid(columns: (auto, auto, 1fr), gutter: 6pt, align: horizon)[
      #severity-badge(f.severity)
    ][
      #text(font: mono-font, size: 8.4pt, fill: c-muted)[#f.id]
    ][
      #align(right)[#status-badge(f.status)]
    ]
    #v(0.35em)
    #text(font: title-font, size: 11.5pt, weight: "bold")[#f.title]
    #v(0.15em)
    #text(font: title-font, size: 8pt, fill: c-brand, tracking: 0.03em)[#upper(f.subsystem)]
    #v(0.4em)
    #if "file" in f and f.file != none [
      #text(font: mono-font, size: 8.6pt, fill: c-muted)[
        #f.file #if "line" in f and f.line != none [ : #f.line ]
      ]
      #v(0.35em)
    ]
    #evidence-label(f.evidence_quality)
    #v(0.4em)
    #line(length: 100%, stroke: 0.3pt + c-rule)
    #v(0.4em)
    #field-line("Evidence", f.evidence)
    #field-line("Failure scenario", f.failure_scenario)
    #field-line("Blast radius", f.at("blast_radius", default: none))
    #field-line("Operational impact", f.at("operational_impact", default: none))
    #field-line("Recommendation", f.recommendation)
    #field-line("Verification needed", f.at("verification_needed", default: none))
    #if "blocks" in f and f.blocks.len() > 0 [
      #text(font: title-font, size: 8.2pt, weight: "bold", fill: c-muted)[BLOCKS: ]
      #text(size: 9.2pt)[#f.blocks.map(b => b.replace("_", " ")).join(", ")]
    ]
  ]
}

// ---- simple key metric tile -------------------------------------------

#let metric-tile(value, label, color: none) = {
  let col = if color == none { c-brand } else { color }
  block(fill: c-bg-panel, radius: 3pt, inset: 10pt, width: 100%)[
    #align(center)[
      #text(font: title-font, size: 26pt, weight: "bold", fill: col)[#value]
      #v(0.15em)
      #text(font: title-font, size: 8pt, fill: c-muted, tracking: 0.03em)[#upper(label)]
    ]
  ]
}
