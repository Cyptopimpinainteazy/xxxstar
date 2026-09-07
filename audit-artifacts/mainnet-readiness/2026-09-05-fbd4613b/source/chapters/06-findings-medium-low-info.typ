#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Findings: Medium, Low & Informational

== Medium-Severity Findings

These #by-severity("Medium").len() findings should be resolved before mainnet hardening is considered complete; most are not blockers for a closed internal testnet.

#for f in by-severity("Medium") [
  #finding-card(f)
]

== Low-Severity and Informational Findings

The remaining #(by-severity("Low").len() + by-severity("Informational").len()) findings are hygiene items, naming/documentation-accuracy notes, or explicitly-noted non-defects (false positives from generic scanning tooling). They are included in full for completeness, condensed into tables rather than full cards since none carry an active exploit scenario.

#table(
  columns: (auto, auto, 2.2fr, 1.6fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*ID*], [*Sev*], [*Title*], [*File*],
  ..for f in by-severity("Low") + by-severity("Informational") {
    (
      [#f.id],
      [#severity-badge(f.severity)],
      [#f.title],
      [#text(font: mono-font, size: 8pt)[#f.file]],
    )
  }
)

#v(0.6em)

#for f in by-severity("Low") [
  #finding-card(f)
]

#for f in by-severity("Informational") [
  #finding-card(f)
]
