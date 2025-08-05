#!/bin/bash
set -e

# Configuration
IDENTITY="Developer ID Application: Insihts, Corp. (RVPAX6PXC6)"
APP_PATH="/Users/7hakurg/Experiements/Code/IDE/actions-runner/_work/oppla_ai_ide/oppla_ai_ide/target/x86_64-apple-darwin/release/bundle/osx/Oppla.app"
ENTITLEMENTS_PATH="/Users/7hakurg/Experiements/Code/IDE/oppla_ide/crates/oppla/resources/oppla.entitlements"

echo "=== Step 1: Check current signatures ==="
codesign -vvv --deep --strict "$APP_PATH" 2>&1 || true
echo ""

echo "=== Step 2: Remove all existing signatures ==="
codesign --remove-signature "$APP_PATH/Contents/MacOS/cli" 2>/dev/null || true
codesign --remove-signature "$APP_PATH/Contents/MacOS/git" 2>/dev/null || true
codesign --remove-signature "$APP_PATH/Contents/MacOS/oppla" 2>/dev/null || true
codesign --remove-signature "$APP_PATH" 2>/dev/null || true
echo "Signatures removed"
echo ""

echo "=== Step 3: Sign individual binaries first ==="
echo "Signing cli..."
codesign --force --timestamp --options runtime --sign "$IDENTITY" "$APP_PATH/Contents/MacOS/cli" -v

echo "Signing git..."
codesign --force --timestamp --options runtime --sign "$IDENTITY" "$APP_PATH/Contents/MacOS/git" -v

echo "Signing oppla binary..."
codesign --force --timestamp --options runtime --entitlements "$ENTITLEMENTS_PATH" --sign "$IDENTITY" "$APP_PATH/Contents/MacOS/oppla" -v
echo ""

echo "=== Step 4: Sign the app bundle ==="
codesign --force --deep --timestamp --options runtime --entitlements "$ENTITLEMENTS_PATH" --sign "$IDENTITY" "$APP_PATH" -v
echo ""

echo "=== Step 5: Verify signatures ==="
codesign -vvv --deep --strict "$APP_PATH"
echo ""

echo "=== Step 6: Check with spctl ==="
spctl -a -vvv "$APP_PATH"
echo ""

echo "=== Step 7: Create DMG ==="
DMG_PATH="/tmp/Oppla-test.dmg"
DMG_SOURCE="/tmp/dmg-source"
rm -rf "$DMG_SOURCE"
mkdir -p "$DMG_SOURCE"
cp -R "$APP_PATH" "$DMG_SOURCE/"
ln -s /Applications "$DMG_SOURCE/Applications"

hdiutil create -volname Oppla -srcfolder "$DMG_SOURCE" -ov -format UDZO "$DMG_PATH"
echo "DMG created at: $DMG_PATH"
echo ""

echo "=== Step 8: Sign DMG ==="
codesign --force --timestamp --options runtime --sign "$IDENTITY" "$DMG_PATH" -v
echo ""

echo "=== Step 9: Submit for notarization ==="
echo "Run this command with your credentials:"
echo "xcrun notarytool submit --wait --key /path/to/your/AuthKey.p8 --key-id YOUR_KEY_ID --issuer YOUR_ISSUER_ID $DMG_PATH"
echo ""

echo "=== Step 10: After notarization succeeds, staple the ticket ==="
echo "xcrun stapler staple $DMG_PATH"