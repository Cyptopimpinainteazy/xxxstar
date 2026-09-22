# x3-lang price-impact / MEV ceilings — implementation record

Date: 2026-09-22
PR: #451

## Implemented

- `TradeRiskPolicy.max_price_impact_bps` / `max_mev_leakage_bps` through
  AST, parser, formatter, semantic validation, IR, and VM.
- `TradingHost::price_impact()` and `TradingHost::mev_leakage()` return
  `Option<MeasuredRisk>` with a named source and basis-point value.
- VM execution fails closed when a policy declares a ceiling and the host
  reports no measurement; it rejects measurements above the ceiling.
- `LegQuoteWindow` carries both measurements.
- `verify_receipt_economics` re-checks both ceilings from the receipt.

## Tests

- execution above price-impact ceiling
- execution missing price-impact measurement
- execution above MEV-leakage ceiling
- replay above price-impact ceiling
- replay missing price-impact measurement

## Remaining

- Source-level guards (`require price_impact <= N`,
  `require mev_leakage <= N`) require widening the measured-unit encoding in
  `x3-lang/spec/opcodes.rs`.
- Real production host adapters must supply the actual measurements.
