# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**⚠️ DO NOT report security vulnerabilities through public GitHub issues.**

### How to Report

Please report security vulnerabilities by emailing: **security@x3-chain.io**

Include the following information:

1. **Description** - A clear description of the vulnerability
2. **Impact** - What an attacker could achieve by exploiting this
3. **Reproduction Steps** - Detailed steps to reproduce the issue
4. **Affected Components** - Which parts of the system are affected
5. **Suggested Fix** - If you have ideas for how to fix it

### What to Expect

- **Acknowledgment**: We will acknowledge your report within 48 hours
- **Assessment**: We will assess the vulnerability within 7 days
- **Fix Timeline**: Critical vulnerabilities will be addressed within 30 days
- **Disclosure**: We will coordinate responsible disclosure with you

### Scope

The following are in scope:

- X3 Kernel pallet (pallet-x3-kernel)
- EVM Adapter (x3-evm-integration)
- SVM Adapter (x3-svm-integration)
- Runtime (x3-chain-runtime)
- Node (x3-chain-node)
- RPC endpoints

### Out of Scope

- Third-party dependencies (report to maintainers)
- Social engineering attacks
- Physical security issues
- Issues already known/reported

### Recognition

We appreciate security researchers and will:

- Credit you in our security advisories (if desired)
- Consider bug bounty rewards for significant findings
- Provide early access to fixes for testing

## Security Best Practices

### For Node Operators

1. Keep your node software up to date
2. Use firewall rules to restrict RPC access
3. Run nodes as non-root users
4. Enable TLS for public endpoints
5. Monitor logs for suspicious activity
6. Back up keys securely (HSM recommended)

### For Developers

1. Never commit private keys
2. Use the authorized accounts system
3. Validate all inputs in smart contracts
4. Test cross-VM interactions thoroughly
5. Follow secure coding guidelines

## Contact

- Security issues: security@x3-chain.io
- General inquiries: contact@x3-chain.io

## PGP Key

```
[PGP public key to be added]
```
