# Release Notes - Oppla v0.201.11

## 🎉 Highlights

This release brings significant improvements to the macOS build and distribution process, ensuring a more reliable and professional installation experience for our users.

## 🔧 Build & Distribution Improvements

### macOS Code Signing & Notarization
- **Enhanced Certificate Verification**: Improved code signing process with comprehensive certificate validation
- **Deep Code Signing**: Implemented proper deep signing for all app bundles and nested binaries
- **Notarization Pipeline**: Streamlined Apple notarization with proper error handling and retry logic
- **Gatekeeper Compliance**: Full verification that DMG installers pass macOS Gatekeeper checks

### DMG Installer Enhancements
- **Simplified DMG Creation**: Reverted to proven, reliable DMG creation method for stability
- **Professional Installer**: Clean, minimalist DMG design with proper app-to-Applications drag-and-drop
- **License Agreement**: Integrated software license agreement into DMG installation flow
- **Universal Binary Support**: Full support for both Intel (x86_64) and Apple Silicon (arm64) architectures

## 🐛 Bug Fixes

- Fixed DMG creation failures in CI/CD pipeline
- Resolved certificate chain validation issues
- Fixed TeamIdentifier verification for proper code signing
- Corrected designated requirements format for notarization
- Removed unnecessary README.md dependencies in DMG creation

## 🏗️ Infrastructure

- **Robust Error Handling**: Added comprehensive error detection and logging throughout build process
- **Keychain Management**: Improved secure keychain creation and certificate import process
- **CI/CD Stability**: Enhanced GitHub Actions workflow reliability for automated builds

## 📦 Technical Details

- Proper handling of Apple Developer ID certificates
- Automated certificate chain import (Apple Root, WWDR, Developer ID G2)
- Secure keychain operations with proper cleanup
- Notarization status tracking and verification
- Stapling of notarization tickets to DMG files

## 🔐 Security

- All binaries are properly code-signed with Developer ID
- Runtime hardening enabled for enhanced security
- Proper entitlements configuration for app capabilities
- Secure handling of signing certificates and notarization credentials

---

## Previous Features (v0.201.8 - v0.201.10)

- **Oppla Branding**: Complete rebranding with new visual identity
- **Edit Predictions**: AI-powered code completion and suggestions
- **Semantic Search**: Cloud-based embeddings for improved code search
- **UI Enhancements**: Harmonized components with extensible registry system
- **Performance**: Optimized app icon sizing and resource loading

---

*Built with care by the Oppla team* 🚀