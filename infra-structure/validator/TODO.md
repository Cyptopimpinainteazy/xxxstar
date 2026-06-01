# Validator Infrastructure TODO

- [ ] Create todo list for all required changes
- [ ] Update Dockerfiles and docker-compose files for consistent build/runtime contract
- [x] Fix healthcheck script and pyproject.toml references
 - [x] Adjust GPU detection logic in cuda_loader.py
 - [x] Update docker-compose manifests for GPU mode handling
 - [ ] Modify CLI verifier selection for signature algorithms
 - [ ] Enforce public-key sizes in validators
 - [ ] Require real Keccak provider and adjust tests
 - [ ] Run tests to ensure changes compile