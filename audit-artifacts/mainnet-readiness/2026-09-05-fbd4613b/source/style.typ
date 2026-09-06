// Page setup, typography, and color palette for the X3 audit booklet.
// Light theme only (print-safe), restrained technical palette.

#let c-ink = rgb("#1a1a2e")
#let c-muted = rgb("#5a5a6e")
#let c-rule = rgb("#c7c7d1")
#let c-bg-panel = rgb("#f4f4f8")
#let c-bg-page = rgb("#ffffff")
#let c-brand = rgb("#1e3a5f")
#let c-brand-light = rgb("#e8eef5")

#let c-critical = rgb("#8c1c13")
#let c-critical-bg = rgb("#fbe9e7")
#let c-high = rgb("#a8460a")
#let c-high-bg = rgb("#fdefe2")
#let c-medium = rgb("#8a6d00")
#let c-medium-bg = rgb("#faf3d9")
#let c-low = rgb("#2e6b32")
#let c-low-bg = rgb("#e8f3e8")
#let c-info = rgb("#2d5a8a")
#let c-info-bg = rgb("#e7eff8")

#let title-font = "Liberation Sans"
#let body-font = "Libertinus Serif"
#let mono-font = "Liberation Mono"

#let doc-title = "X3 Atomic Star: The Road to Mainnet"
#let doc-subtitle = "Evidence-Based Architecture Audit, Gap Analysis, and Production Completion Blueprint"
#let audit-commit = "fbd4613bd8769ac7422278fae441af1b302a1c88"
#let audit-commit-short = "fbd4613b"
#let audit-date = "2026-09-05"
#let audit-branch = "master"

#let setup-page(body) = {
  set document(title: doc-title, author: "Automated audit agent (Claude, Anthropic)")
  set text(font: body-font, size: 10.3pt, fill: c-ink, lang: "en")
  set par(justify: true, leading: 0.62em, first-line-indent: 0em)

  set page(
    paper: "a4",
    margin: (top: 2.6cm, bottom: 2.4cm, x: 2.2cm),
    header: context {
      let pg = counter(page).get().first()
      if pg > 2 [
        #set text(8pt, fill: c-muted, font: title-font)
        #grid(columns: (1fr, 1fr), align: (left, right))[
          X3 ATOMIC STAR — MAINNET READINESS AUDIT
        ][
          commit #audit-commit-short · #audit-date
        ]
        #v(-0.55em)
        #line(length: 100%, stroke: 0.4pt + c-rule)
      ]
    },
    footer: context {
      let pg = counter(page).get().first()
      if pg > 2 [
        #line(length: 100%, stroke: 0.4pt + c-rule)
        #v(0.15em)
        #set text(8pt, fill: c-muted, font: title-font)
        #grid(columns: (1fr, 1fr), align: (left, right))[
          Confidential — Internal Testnet Candidate Audit
        ][
          Page #counter(page).display() of #context counter(page).final().first()
        ]
      ]
    },
    numbering: "1",
  )

  set heading(numbering: "1.1")
  show heading.where(level: 1): it => {
    pagebreak(weak: true)
    v(0.3em)
    block(text(font: title-font, size: 9pt, fill: c-brand, tracking: 0.15em)[CHAPTER #context counter(heading).display()])
    v(0.15em)
    block(text(font: title-font, size: 22pt, weight: "bold", fill: c-ink)[#it.body])
    v(0.6em)
    line(length: 100%, stroke: 1.2pt + c-brand)
    v(0.8em)
  }
  show heading.where(level: 2): it => {
    v(0.9em)
    block(text(font: title-font, size: 14pt, weight: "bold", fill: c-brand)[#it])
    v(0.3em)
  }
  show heading.where(level: 3): it => {
    v(0.6em)
    block(text(font: title-font, size: 11.5pt, weight: "bold", fill: c-ink)[#it])
    v(0.2em)
  }

  show raw: set text(font: mono-font, size: 9pt)
  show raw.where(block: true): it => block(
    fill: c-bg-panel, inset: 8pt, radius: 3pt, width: 100%, stroke: 0.4pt + c-rule, it
  )
  show link: set text(fill: c-brand)

  set figure(numbering: "1")
  show figure.caption: it => text(size: 8.6pt, fill: c-muted, style: "italic")[#it]

  set table(stroke: 0.5pt + c-rule)
  show table.cell.where(y: 0): set text(weight: "bold", fill: white)
  show table.cell.where(y: 0): set align(left + horizon)

  body
}
