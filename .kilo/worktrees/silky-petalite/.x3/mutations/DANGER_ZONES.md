# X3 Danger Zones

Agents must stop and request audit-mode review before broad changes to:

- runtime/**
- pallets/**
- crates/*bridge*/**
- crates/*vm*/**
- crates/*asset*/**
- contracts/**
- X3-contracts/**
- genesis/**
- chain-spec/**
- treasury config
- validator keys
- deployment addresses
- bridge admin config

Rules:
- patch small
- add tests
- document rollback
- update X3_RISK_REGISTER.md
- update PATCH_LOG.md
- do not make broad rewrites
