#!/usr/bin/env bash

# Corrected signing function for bundle-mac script
function sign_app_binaries() {
    local app_path=$1
    local architecture=$2
    local architecture_dir=$3
    echo "====== Signing app binaries for $architecture ======"

    if [[ $can_code_sign = true ]]; then
        echo "Code signing binaries"
        
        # IMPORTANT: Sign from innermost to outermost, WITHOUT --deep flag
        
        # 1. Sign individual binaries first (no --deep needed for single binaries)
        echo "Signing cli binary..."
        /usr/bin/codesign --force --timestamp --options runtime \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/cli" -v
        
        echo "Signing git binary..."
        /usr/bin/codesign --force --timestamp --options runtime \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/git" -v
        
        echo "Signing oppla binary..."
        /usr/bin/codesign --force --timestamp --options runtime \
            --entitlements crates/oppla/resources/oppla.entitlements \
            --sign "$IDENTITY" \
            "${app_path}/Contents/MacOS/oppla" -v
        
        # 2. Sign any frameworks or nested bundles if they exist
        if [ -d "${app_path}/Contents/Frameworks" ]; then
            find "${app_path}/Contents/Frameworks" -name "*.framework" -o -name "*.dylib" | while read -r framework; do
                echo "Signing framework: $framework"
                /usr/bin/codesign --force --timestamp --options runtime \
                    --sign "$IDENTITY" "$framework" -v
            done
        fi
        
        # 3. Finally sign the app bundle with explicit designated requirement
        # Get the bundle ID from Info.plist
        BUNDLE_ID=$(defaults read "${app_path}/Contents/Info.plist" CFBundleIdentifier 2>/dev/null || echo "ai.oppla.Oppla")
        
        echo "Signing app bundle with bundle ID: $BUNDLE_ID"
        /usr/bin/codesign --force --timestamp --options runtime \
            --entitlements crates/oppla/resources/oppla.entitlements \
            --sign "$IDENTITY" \
            -r="designated => identifier \"$BUNDLE_ID\" and anchor apple generic" \
            "${app_path}" -v

        # 4. Comprehensive verification
        echo "Verifying signatures..."
        
        # Check basic signature
        if ! codesign -vvv "${app_path}" 2>&1; then
            echo "❌ ERROR: Basic signature verification failed!"
            exit 1
        fi
        
        # Check deep signature (all nested code)
        if ! codesign -vvv --deep --strict "${app_path}" 2>&1; then
            echo "❌ ERROR: Deep signature verification failed!"
            exit 1
        fi
        
        # Check designated requirement
        if ! codesign --verify --deep --strict --verbose=4 "${app_path}" 2>&1 | grep -q "satisfies its Designated Requirement"; then
            echo "❌ ERROR: Designated requirement not satisfied!"
            exit 1
        fi
        
        # Check hardened runtime
        if ! codesign -dvvv "${app_path}" 2>&1 | grep -q "flags.*runtime"; then
            echo "❌ ERROR: Hardened runtime not enabled!"
            exit 1
        fi
        
        # Check timestamp
        if ! codesign -dvvv "${app_path}" 2>&1 | grep -q "Timestamp"; then
            echo "❌ ERROR: Timestamp missing!"
            exit 1
        fi
        
        echo "✅ All signature checks passed!"
        
    else
        # ... rest of the else block remains the same ...
    fi
}