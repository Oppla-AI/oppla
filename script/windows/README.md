# Windows Bundling for Oppla

This directory contains the new Windows bundling system that follows the same patterns as the Mac and Linux bundling scripts. Unlike the existing `bundle-windows.ps1` (which creates installers using Inno Setup), this system creates portable .zip archives similar to how Mac creates .dmg files and Linux creates .tar.gz archives. This script is written in bash for cross-platform consistency.

## Overview

The Windows bundling system:

- **Creates portable application directories** - All binaries and dependencies are packaged in a self-contained directory structure
- **Generates .zip archives** - Similar to .dmg (Mac) and .tar.gz (Linux) archives for distribution
- **Follows the same patterns** - Consistent with Mac and Linux bundling in terms of environment variables, channel handling, version management
- **Skips code signing by default** - Since code signing keys aren't available yet, this focuses on creating unsigned portable builds
- **Handles debug symbols** - Compresses and uploads PDB files for crash reporting
- **Supports multiple channels** - Stable, Preview, Nightly, and Dev builds

## File Structure

```
script/windows/
├── bundle.ps1                    # Main bundling script
├── README.md                     # This file
├── lib/
│   ├── blob-store.ps1           # Azure blob storage upload functions
│   ├── compress-file.ps1         # File compression utilities
│   ├── generate-licenses.ps1     # License generation script
│   └── get-crate-version.ps1     # Version extraction utility
├── resources/                    # Windows-specific resources (icons, etc.)
└── target/                       # Output directory (created during build)
```

## Usage

### Basic Usage

```bash
./script/windows/bundle
```

### With Options

```bash
# Get help
./script/windows/bundle --help

# Create local build only (no archive)
./script/windows/bundle --local-only

# Open result directory after build
./script/windows/bundle --open-result

# Specify custom bundle name
./script/windows/bundle --bundle-name "Oppla-Custom"
```

### Examples

```bash
# Standard build for distribution
./script/windows/bundle

# Local development build
./script/windows/bundle --local-only --open-result

# Build with custom name and open result
./script/windows/bundle --bundle-name "MyOppla" --open-result
```

## Output

### Portable Application Structure

The script creates a portable application directory:

```
oppla[-channel]/
├── bin/
│   └── oppla.exe                  # CLI executable
├── lib/
│   ├── zed_explorer_command_injector.dll
│   └── [vc-runtime dlls...]       # Visual C++ runtime dependencies
├── libexec/
│   ├── oppla-editor.exe           # Main application
│   └── auto_update_helper.exe     # Update helper
├── licenses.md                    # License information
└── oppla.ico                      # Application icon
```

### Archive Files

- **Release builds**: `oppla-windows-{arch}.zip`
- **Dev builds**: `oppla-{commit}-windows-{arch}.zip`
- **Remote server**: `oppla-remote-server-windows-{arch}.gz`
- **Debug symbols**: `{component}-{version}-{channel}.pdb.zip`

### Architecture Support

The script automatically detects the target architecture:
- `x64` for x86_64-pc-windows-msvc
- `x86` for i686-pc-windows-msvc  
- `arm64` for aarch64-pc-windows-msvc

## Environment Variables

### Required Variables

- `OPPLA_RELEASE_CHANNEL` / `ZED_RELEASE_CHANNEL` - Release channel (stable, preview, nightly, dev)
- `RELEASE_VERSION` - Application version

### Optional Variables

- `RUST_TARGET_DIR` - Custom cargo target directory

### Debug Symbol Upload Variables

For uploading debug symbols to Azure blob storage:
- `AZURE_TENANT_ID`
- `AZURE_CLIENT_ID` 
- `AZURE_CLIENT_SECRET`

## Comparison with bundle-windows.ps1

| Aspect | New bundle | Original bundle-windows.ps1 |
|--------|---------------|----------------------------|
| Output Format | Portable .zip archives | Inno Setup .exe installers |
| Code Signing | Skipped (not implemented) | Full code signing with Azure |
| Distribution | Archive-based distribution | Installer-based distribution |
| Dependencies | Self-contained with VC runtime | System dependencies + installers |
| Target Use | Development, testing, portable builds | Production distribution |
| Apple/Notarization | N/A | Windows AppX packaging |
| Script Language | Bash | PowerShell |

## Build Process

1. **Environment Setup** - Sets bundle flags and release channel
2. **License Generation** - Creates license files using cargo-about
3. **Binary Compilation** - Builds all required Rust binaries:
   - `oppla` (main application)
   - `cli` (command-line interface)
   - `auto_update_helper` (update mechanism)
   - `explorer_command_injector.dll` (Windows integration)
   - `remote_server` (collaboration server)
4. **Dependency Analysis** - Copies required Visual C++ runtime DLLs
5. **Resource Copying** - Copies icons and Windows-specific resources
6. **Debug Symbol Processing** - Compresses PDB files for upload
7. **Archive Creation** - Creates final .zip distribution archive
8. **Cleanup** - Removes temporary files and directories

## Channel Support

### Stable (`oppla`)
- Production-ready builds
- No channel suffix in filenames
- Uses stable-specific features when available

### Preview (`oppla-preview`) 
- Beta testing builds
- `-preview` suffix in filenames
- Uses preview-specific features

### Nightly (`oppla-nightly`)
- Daily development builds
- `-nightly` suffix in filenames
- Latest features and changes

### Dev (`oppla-dev`)
- Development builds
- `-dev` suffix in filenames
- Commit hash included in archive name

## Requirements

### System Requirements
- Windows 10 or later
### Bash 4.0 or later
- Rust toolchain
- Visual Studio Build Tools (for C++ dependencies)

### Development Dependencies
- `cargo` - Rust package manager
- `cargo-about` - License generation
- Git - For commit hash information

### Build Dependencies
