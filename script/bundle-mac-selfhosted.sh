#!/usr/bin/env bash

# Simplified bundle-mac for self-hosted runners
# Uses system keychain instead of creating temporary keychains

set -euo pipefail
source script/lib/blob-store.sh

build_flag="--release"
target_dir="release"
open_result=false
local_arch=false
local_only=false
local_install=false
bundle_name=""

# Certificate identity - must be installed in system keychain
IDENTITY="Developer ID Application: Insihts, Corp. (RVPAX6PXC6)"
APPLE_NOTARIZATION_TEAM="RVPAX6PXC6"

# Function for displaying help info
help_info() {
  echo "
Usage: ${0##*/} [options] [bundle_name]
Build the application bundle for macOS (self-hosted runner version).

Options:
  -d    Compile in debug mode
  -l    Compile for local architecture only.
  -o    Open dir with the resulting DMG or launch the app itself in local mode.
  -i    Install the resulting DMG into /Applications in local mode. Noop without -l.
  -h    Display this help and exit.
  "
}

while getopts 'dloih' flag
do
    case "${flag}" in
        o) open_result=true;;
        d)
            export CARGO_INCREMENTAL=true
            export CARGO_BUNDLE_SKIP_BUILD=true
            build_flag="";
            target_dir="debug"
            ;;
        l)
            export CARGO_INCREMENTAL=true
            export CARGO_BUNDLE_SKIP_BUILD=true
            local_arch=true
            local_only=true
            ;;
        i) local_install=true;;
        h)
           help_info
           exit 0
           ;;
    esac
done

shift $((OPTIND-1))

if [[ $# -gt 0 ]]; then
    if [ "$1" ]; then
        bundle_name=$1
    fi
fi

# Get release channel
pushd crates/oppla
channel=$(<RELEASE_CHANNEL)
export ZED_RELEASE_CHANNEL="${channel}"
popd

export ZED_BUNDLE=true

# Check cargo-bundle
cargo_bundle_version=$(cargo -q bundle --help 2>&1 | head -n 1 || echo "")
if [ "$cargo_bundle_version" != "cargo-bundle v0.6.1-zed" ]; then
    cargo install cargo-bundle --git https://github.com/zed-industries/cargo-bundle.git --branch zed-deploy
fi

# Deal with versions of macOS that don't include libstdc++ headers
export CXXFLAGS="-stdlib=libc++"

version_info=$(rustc --version --verbose)
host_line=$(echo "$version_info" | grep host)
local_target_triple=${host_line#*: }

# Generate the licenses
script/generate-licenses

# Build
if [ "$local_arch" = true ]; then
    echo "Building for local target only."
    cargo build ${build_flag} --package oppla --package cli --package remote_server
else
    echo "Compiling oppla binaries"
    cargo build ${build_flag} --package oppla --package cli --target aarch64-apple-darwin --target x86_64-apple-darwin
    cargo build ${build_flag} --package remote_server --target aarch64-apple-darwin --target x86_64-apple-darwin
fi

# Create application bundle
echo "Creating application bundle"
pushd crates/oppla
cp Cargo.toml Cargo.toml.backup
sed \
    -i.backup \
    "s/package.metadata.bundle-${channel}/package.metadata.bundle/" \
    Cargo.toml

if [ "$local_arch" = true ]; then
    app_path=$(cargo bundle ${build_flag} --select-workspace-root | xargs)
else
    app_path_x64=$(cargo bundle ${build_flag} --target x86_64-apple-darwin --select-workspace-root | xargs)
    app_path_aarch64=$(cargo bundle ${build_flag} --target aarch64-apple-darwin --select-workspace-root | xargs)
    app_path=$app_path_x64
fi

mv Cargo.toml.backup Cargo.toml
popd
echo "Bundled ${app_path}"

# Check if we can code sign
can_code_sign=false
if security find-identity -v -p codesigning | grep -q "$IDENTITY"; then
    echo "✅ Signing identity found in keychain"
    can_code_sign=true
    
    # Also check for notarization credentials
    if [[ -n "${APPLE_NOTARIZATION_KEY:-}" && -n "${APPLE_NOTARIZATION_KEY_ID:-}" && -n "${APPLE_NOTARIZATION_ISSUER_ID:-}" ]]; then
        echo "✅ Notarization credentials available"
    else
        echo "⚠️  Warning: Notarization credentials not found. DMG won't be notarized."
    fi
else
    echo "⚠️  Warning: Signing identity not found in keychain"
    echo "To install: security import certificate.p12 -P password -T /usr/bin/codesign"
fi

# Git version for bundling
GIT_VERSION="v2.43.3"
GIT_VERSION_SHA="fa29823"

function download_and_unpack() {
    local url=$1
    local path_to_unpack=$2
    local target_path=$3
    
    temp_dir=$(mktemp -d)
    curl --silent --fail --location "$url" | tar -xvz -C "$temp_dir" -f - $path_to_unpack
    mv "$temp_dir/$path_to_unpack" "$target_path"
    rm -rf "$temp_dir"
}

function download_git() {
    local architecture=$1
    local target_binary=$2
    
    tmp_dir=$(mktemp -d)
    pushd "$tmp_dir"
    
    case "$architecture" in
        aarch64-apple-darwin)
            download_and_unpack "https://github.com/desktop/dugite-native/releases/download/${GIT_VERSION}/dugite-native-${GIT_VERSION}-${GIT_VERSION_SHA}-macOS-arm64.tar.gz" bin/git ./git
            ;;
        x86_64-apple-darwin)
            download_and_unpack "https://github.com/desktop/dugite-native/releases/download/${GIT_VERSION}/dugite-native-${GIT_VERSION}-${GIT_VERSION_SHA}-macOS-x64.tar.gz" bin/git ./git
            ;;
        *)
            echo "Unsupported architecture: $architecture"
            exit 1
            ;;
    esac
    
    popd
    mv "${tmp_dir}/git" "${target_binary}"
    rm -rf "$tmp_dir"
}

function prepare_binaries() {
    local architecture=$1
    local app_path=$2
    
    echo "Unpacking dSYMs for $architecture"
    exe_path="target/${architecture}/${target_dir}/oppla"
    if ! dsymutil --flat "${exe_path}" 2> target/dsymutil.log; then
        echo "dsymutil failed"
        cat target/dsymutil.log
        exit 1
    fi
    
    uuid=$(dwarfdump --uuid "${exe_path}" | cut -d ' ' -f 2 | tr 'A-F' 'a-f')
    version="$(cargo metadata --no-deps --manifest-path crates/oppla/Cargo.toml --offline --format-version=1 | jq -r '.packages | map(select(.name == "oppla"))[0].version')"
    if [ "$channel" == "nightly" ]; then
        version="$version-$(git rev-parse --short HEAD)"
    fi
    
    rm -f target/${architecture}/${target_dir}/oppla.dwarf.gz
    gzip -f target/${architecture}/${target_dir}/oppla.dwarf
    
    # Upload debug symbols if credentials are available
    if [[ -n "${DIGITALOCEAN_SPACES_ACCESS_KEY:-}" && -n "${DIGITALOCEAN_SPACES_SECRET_KEY:-}" ]]; then
        upload_to_blob_store_public \
            "oppla-debug-symbols" \
            target/${architecture}/${target_dir}/oppla.dwarf.gz \
            "by-uuid/${uuid}.dwarf.gz"
    fi
    
    cp target/${architecture}/${target_dir}/oppla "${app_path}/Contents/MacOS/oppla"
    cp target/${architecture}/${target_dir}/cli "${app_path}/Contents/MacOS/cli"
    
    echo "Downloading git binary for $architecture"
    download_git "${architecture}" "${app_path}/Contents/MacOS/git"
    
    echo "Copying provisioning profile"
    cp crates/oppla/contents/$channel/embedded.provisionprofile "${app_path}/Contents/"
    
    rm -rf "${app_path}/Contents/Frameworks"
    mkdir -p "${app_path}/Contents/Frameworks"
}

function sign_app_binaries() {
    local app_path=$1
    local architecture=$2
    local architecture_dir=$3
    echo "====== Signing app binaries for $architecture ======"
    
    if [[ $can_code_sign = true ]]; then
        echo "Code signing binaries using system keychain"
        
        # Sign binaries individually with --deep
        echo "Signing cli binary..."
        codesign --deep --force --timestamp --options runtime \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/cli" -v
        
        echo "Signing git binary..."
        codesign --deep --force --timestamp --options runtime \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/git" -v
        
        echo "Signing oppla binary..."
        codesign --deep --force --timestamp --options runtime \
            --entitlements crates/oppla/resources/oppla.entitlements \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/oppla" -v
        
        # Sign any frameworks if they exist
        if [ -d "${app_path}/Contents/Frameworks" ]; then
            find "${app_path}/Contents/Frameworks" -name "*.framework" -o -name "*.dylib" | while read -r framework; do
                echo "Signing framework: $framework"
                codesign --deep --force --timestamp --options runtime \
                    --sign "$IDENTITY" "$framework" -v
            done
        fi
        
        # Sign the app bundle (without --deep to preserve nested signatures)
        BUNDLE_ID=$(defaults read "${app_path}/Contents/Info.plist" CFBundleIdentifier 2>/dev/null || echo "ai.oppla.Oppla")
        echo "Signing app bundle with bundle ID: $BUNDLE_ID"
        codesign --force --timestamp --options runtime \
            --entitlements crates/oppla/resources/oppla.entitlements \
            --sign "$IDENTITY" \
            "${app_path}" -v
        
        # Verify signature
        echo "Verifying signatures..."
        if codesign -vvv --deep --strict "${app_path}" 2>&1; then
            echo "✅ Code signature verification passed!"
        else
            echo "⚠️  WARNING: Local verification had warnings, but continuing..."
            echo "Notarization will provide the final verification"
        fi
    else
        echo "⚠️  Signing with ad-hoc signature (not for distribution)"
        codesign --force --deep --sign - "${app_path}" -v
    fi
    
    # Handle local builds
    if [[ "$target_dir" = "debug" && "$local_only" = false ]]; then
        if [ "$open_result" = true ]; then
            open "$app_path"
        else
            echo "Created application bundle: $app_path"
        fi
        exit 0
    fi
    
    # Set bundle name
    if [ -z "$bundle_name" ]; then
        bundle_name=$(basename "$app_path")
    else
        if [[ "$bundle_name" != *.app ]]; then
            bundle_name="$bundle_name.app"
        fi
    fi
    
    # Local installation
    if [ "$local_only" = true ]; then
        if [ "$local_install" = true ]; then
            rm -rf "/Applications/$bundle_name"
            mv "$app_path" "/Applications/$bundle_name"
            echo "Installed: /Applications/$bundle_name"
            if [ "$open_result" = true ]; then
                open "/Applications/$bundle_name"
            fi
        else
            if [ "$open_result" = true ]; then
                open "$app_path"
            fi
        fi
    else
        # Create DMG for distribution
        dmg_target_directory="target/${architecture_dir}/${target_dir}"
        dmg_source_directory="${dmg_target_directory}/dmg"
        dmg_file_path="${dmg_target_directory}/Oppla.dmg"
        xcode_bin_dir_path="$(xcode-select -p)/usr/bin"
        
        rm -rf ${dmg_source_directory}
        mkdir -p ${dmg_source_directory}
        mv "${app_path}" "${dmg_source_directory}"
        
        echo "Adding symlink to /Applications"
        ln -s /Applications ${dmg_source_directory}
        
        echo "Creating DMG"
        hdiutil create -volname Oppla -srcfolder "${dmg_source_directory}" -ov -format UDZO "${dmg_file_path}"
        
        # Remove symlink to avoid issues
        rm ${dmg_source_directory}/Applications
        
        # Add license if npm is available
        if command -v npm &> /dev/null; then
            echo "Adding license agreement to DMG"
            npm install --global dmg-license minimist
            dmg-license script/terms/terms.json "${dmg_file_path}"
        fi
        
        if [[ $can_code_sign = true ]]; then
            echo "Signing DMG"
            codesign --deep --force --timestamp --options runtime --sign "$IDENTITY" "${dmg_file_path}" -v
            
            # Notarize if credentials are available
            if [[ -n "${APPLE_NOTARIZATION_KEY:-}" && -n "${APPLE_NOTARIZATION_KEY_ID:-}" && -n "${APPLE_NOTARIZATION_ISSUER_ID:-}" ]]; then
                echo "Notarizing DMG with Apple"
                notarization_key_file=$(mktemp)
                echo "$APPLE_NOTARIZATION_KEY" > "$notarization_key_file"
                
                "${xcode_bin_dir_path}/notarytool" submit "${dmg_file_path}" \
                    --key "$notarization_key_file" \
                    --key-id "$APPLE_NOTARIZATION_KEY_ID" \
                    --issuer "$APPLE_NOTARIZATION_ISSUER_ID" \
                    --wait
                
                rm "$notarization_key_file"
                
                echo "Stapling notarization ticket"
                "${xcode_bin_dir_path}/stapler" staple "${dmg_file_path}"
            fi
        fi
        
        if [ "$open_result" = true ]; then
            open $dmg_target_directory
        fi
    fi
}

function sign_binary() {
    local binary_path=$1
    
    if [[ $can_code_sign = true ]]; then
        echo "Code signing $binary_path"
        codesign --deep --force --timestamp --options runtime \
            --entitlements crates/oppla/resources/oppla.entitlements \
            --sign "$IDENTITY" "${binary_path}" -v
    fi
}

# Main execution
if [ "$local_arch" = true ]; then
    sign_app_binaries "$app_path" "$local_target_triple" "$local_target_triple"
    sign_binary "target/release/remote_server"
else
    # Prepare binaries for both architectures
    prepare_binaries "aarch64-apple-darwin" "$app_path_aarch64"
    prepare_binaries "x86_64-apple-darwin" "$app_path_x64"
    
    # Sign both architectures
    sign_app_binaries "$app_path_x64" "x86_64-apple-darwin" "x86_64-apple-darwin"
    sign_app_binaries "$app_path_aarch64" "aarch64-apple-darwin" "aarch64-apple-darwin"
    
    # Sign remote servers
    sign_binary "target/x86_64-apple-darwin/release/remote_server"
    sign_binary "target/aarch64-apple-darwin/release/remote_server"
    
    # Create gzipped remote servers
    gzip -f --stdout --best target/x86_64-apple-darwin/release/remote_server > target/zed-remote-server-macos-x86_64.gz
    gzip -f --stdout --best target/aarch64-apple-darwin/release/remote_server > target/zed-remote-server-macos-aarch64.gz
fi

# Upload to Sentry if available
if command -v sentry-cli >/dev/null 2>&1 && [[ -n "${SENTRY_AUTH_TOKEN:-}" ]]; then
    echo "Uploading debug symbols to Sentry..."
    sentry-cli debug-files upload --include-sources --wait -p oppla -o oppla-dev \
        "target/x86_64-apple-darwin/${target_dir}/" \
        "target/aarch64-apple-darwin/${target_dir}/"
fi

echo ""
echo "✅ Bundle complete!"