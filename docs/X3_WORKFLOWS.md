# X3 Workflows

Summaries of all workflow files in `.cline/workflows/`.

## 00-start-task.md
Pre-work snapshot. Run `scripts/x3-pre-task.sh`, read status/tasks docs, state goal with proof criteria.

## 01-implement-feature.md
Full feature implementation pipeline: source → tests → wiring → proof → docs.

## 02-fix-bug.md
Bugfix pipeline: reproduce with test, fix source, verify, regression check, proof.

## 03-refactor-safely.md
Safe refactoring: baseline tests, structural change, verify identical pass/fail counts.

## 04-add-tests.md
Test addition: map untested paths, write real assertions, verify, no test weakening.

## 05-security-review.md
Security review: auth, keys, signatures, bridges, rollback, replay protection audit.

## 06-cross-vm-review.md
Cross-VM review: atomicity, timeout/refund, finality, replay, failure injection.

## 07-mainnet-readiness-review.md
Mainnet readiness: all production gates, full test suite, security + cross-VM reviews, stub clean.

## 08-final-proof-report.md
Mandatory session output: claim, status bar, files, proof, proven/not-proven, blockers, next 10 tasks, verdict.