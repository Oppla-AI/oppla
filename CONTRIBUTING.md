# Contributing to Oppla

Thank you for your interest in contributing to Oppla! We're building an AI-powered development ecosystem that helps developers know what to build and build it right.

All activity in Oppla forums is subject to our [Code of Conduct](./CODE_OF_CONDUCT.md). Additionally, contributors must sign our Contributor License Agreement before their contributions can be merged.

## About This Project

Oppla is a fork of [Zed](https://github.com/zed-industries/zed), extending it with AI-powered features for opportunity identification and contextual code generation. We maintain compatibility with Zed's core architecture while adding our unique capabilities.

## Contribution Ideas

Looking for ways to contribute? Check out:

- **AI Features**: Help improve our opportunity identification and code generation capabilities
- **Editor Enhancements**: Contribute to the core editor functionality inherited from Zed
- **Extensions**: Develop extensions that leverage Oppla's AI capabilities
- **Documentation**: Help improve our documentation and examples
- **Bug Fixes**: Check our [issue tracker](https://github.com/Oppla-AI/oppla/issues) for reported bugs

For adding themes or language support, check out our [docs on developing extensions](https://oppla.ai/docs/extensions).

## Proposing Changes

The best way to propose a change is to [start a discussion on our GitHub repository](https://github.com/Oppla-AI/oppla/discussions).

### 1. Problem Statement
Write a clear and brief description of the problem you want to solve. Focus on the "what" and "why" rather than the "how".

### 2. Solution Proposal
Describe your proposed solution, including:
- How it addresses the problem
- Integration with existing AI features (if applicable)
- Pros and cons of your approach
- Any potential impacts on performance or user experience

### 3. Early Engagement
By discussing your ideas early, we can provide feedback and ensure your contribution aligns with Oppla's roadmap and vision.

## Development Process

### Setting Up Your Environment

1. Fork the repository
2. Clone your fork locally
3. Copy the example environment files and configure them:
   - `crates/collab/.env.toml` - Collaboration server configuration
4. Follow the platform-specific build instructions in `docs/src/development/`

### Code Style and Standards

- Follow Rust best practices and idioms
- Ensure your code passes `./script/clippy`
- Add tests for new functionality
- Document public APIs and complex logic
- Consider AI feature integration where appropriate

### Testing

- Write unit tests for new functionality
- Test AI features with various inputs and edge cases
- Ensure backward compatibility with Zed's core features
- Run the full test suite before submitting PRs

## Pull Request Guidelines

### Tips for Getting Your PR Merged

- **Small, focused PRs**: Break large changes into smaller, reviewable chunks
- **Clear descriptions**: Explain what your PR does and why
- **Test coverage**: Include tests for new functionality
- **Documentation**: Update relevant documentation
- **AI integration**: Consider how new features can leverage or enhance AI capabilities
- **Performance**: Ensure your changes don't negatively impact performance

### PR Process

1. Create a feature branch from `main`
2. Make your changes following our guidelines
3. Test thoroughly
4. Submit a PR with a clear description
5. Address review feedback promptly
6. Once approved, we'll merge your contribution

## Architecture Overview

Oppla extends Zed's architecture with AI capabilities:

### Core Components (from Zed)
- [`gpui`](/crates/gpui) - GPU-accelerated UI framework
- [`editor`](/crates/editor) - Core editor functionality
- [`project`](/crates/project) - Project and file management
- [`workspace`](/crates/workspace) - Workspace state management
- [`lsp`](/crates/lsp) - Language Server Protocol support
- [`language`](/crates/language) - Language understanding and syntax
- [`collab`](/crates/collab) - Collaboration server
- [`theme`](/crates/theme) - Theming system
- [`ui`](/crates/ui) - UI components and patterns

### Oppla AI Extensions
- AI-powered opportunity identification
- Contextual code generation
- Smart prioritization systems
- Integration with AI models

## Community and Support

- **Discord**: [Join our Discord Server](https://discord.gg/KZJD9WqCkS)
- **Discussions**: [GitHub Discussions](https://github.com/Oppla-AI/oppla/discussions)
- **Bug Reports**: [Issue Tracker](https://github.com/Oppla-AI/oppla/issues)
- **Website**: [oppla.ai](https://oppla.ai)
- **Documentation**: [oppla.ai/docs](https://oppla.ai/docs)

## Recognition

We acknowledge and thank the [Zed team](https://github.com/zed-industries/zed) for creating the excellent foundation upon which Oppla is built. Many of the core editor features and architecture decisions come from their work.

## License

By contributing to Oppla, you agree that your contributions will be licensed under the same terms as the project. See our LICENSE files for details.

---

We're excited to have you contribute to Oppla! Together, we're building the future of AI-powered development.