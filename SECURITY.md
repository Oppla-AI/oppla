# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Currently supported versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.x.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of Oppla seriously. If you have discovered a security vulnerability, please follow these steps:

### How to Report

1. **DO NOT** open a public issue on GitHub
2. Email your findings to security@oppla.ai (or your security email)
3. Include the following information:
   - Type of vulnerability
   - Full paths of source file(s) related to the vulnerability
   - Location of the affected source code (tag/branch/commit or direct URL)
   - Step-by-step instructions to reproduce the issue
   - Proof-of-concept or exploit code (if possible)
   - Impact of the issue

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 48 hours
- **Initial Assessment**: Within 7 days, we will provide an initial assessment of the report
- **Fix Timeline**: We will work on a fix and coordinate with you on the disclosure timeline
- **Credit**: We will credit you for the discovery (unless you prefer to remain anonymous)

### Security Best Practices for Contributors

When contributing to Oppla:

1. **Never commit secrets**: API keys, passwords, tokens, etc.
2. **Use environment variables**: For sensitive configuration
3. **Validate inputs**: Always validate and sanitize user inputs
4. **Keep dependencies updated**: Regularly update dependencies to patch known vulnerabilities
5. **Follow secure coding practices**: Use the principle of least privilege

## Security Features

Oppla includes several security features:

- Sandboxed extension execution
- Encrypted communication for collaboration features
- Secure credential storage using OS keychain
- Regular security audits of dependencies

## Disclosure Policy

When we receive a security report, we will:

1. Confirm the problem and determine affected versions
2. Audit code to find similar problems
3. Prepare fixes for all supported releases
4. Release the fixes as soon as possible

Thank you for helping keep Oppla and our users safe!