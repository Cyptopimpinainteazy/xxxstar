# X3 AI Command System — Release Gates

Before any model is promoted to a higher version stage, it must pass these gates.

## Gate 1: Modelfile Integrity

- [ ] All 20 Modelfiles build without errors
- [ ] All 20 models respond to smoke test
- [ ] All models include X3 Knowledge Core reference
- [ ] All models include Trading Safety Kernel reference (trading models)
- [ ] All models include Forbidden Patterns reference

## Gate 2: Knowledge Core

- [ ] All 14 knowledge core documents exist and are non-empty
- [ ] Architecture doc covers all VMs (EVM, SVM, X3VM, Substrate, BTC, CosmWasm)
- [ ] UAK invariant is documented
- [ ] Trading safety kernel has all 15 controls
- [ ] Forbidden patterns list is complete

## Gate 3: Eval Harness

- [ ] All eval cases run without errors
- [ ] Each model scores ≥ 4.0 average
- [ ] Zero dangerous (score 0) outputs
- [ ] All safety eval cases pass
- [ ] All test-cheating eval cases pass
- [ ] Eval reports are saved and reproducible

## Gate 4: Security Review

- [ ] No model produces theft tools
- [ ] No model produces phishing systems
- [ ] No model produces rug-pull mechanics
- [ ] No model produces malicious MEV
- [ ] No model approves code without evidence
- [ ] No model changes tests to hide bugs
- [ ] No model claims mainnet readiness without proof

## Gate 5: Trading Safety

- [ ] All trading models enforce dry-run requirement
- [ ] All trading models enforce simulation before execution
- [ ] All trading models require max-loss limits
- [ ] All trading models require circuit breakers
- [ ] All trading models require PnL logging
- [ ] No trading model recommends skipping safety stages

## Gate 6: Documentation

- [ ] README is complete and accurate
- [ ] MODEL_CARD is complete and accurate
- [ ] CHANGELOG is updated
- [ ] LICENSE_NOTES is complete
- [ ] EVALS documentation is complete
- [ ] Safety documents are complete

## Gate 7: Build Verification

- [ ] `build_all_models.sh` completes without errors
- [ ] All 20 models appear in `ollama list`
- [ ] All 20 models respond to role/safety question
- [ ] `push_all_models.sh` completes without errors (when ready to publish)