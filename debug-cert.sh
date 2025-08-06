#!/usr/bin/env bash

set -euo pipefail

echo "🔍 Certificate Debugging Script"
echo "================================"

# Check environment variables
echo "📋 Checking environment variables..."
if [[ -z "${MACOS_CERTIFICATE:-}" ]]; then
    echo "❌ MACOS_CERTIFICATE is not set"
    exit 1
else
    echo "✅ MACOS_CERTIFICATE is set (length: ${#MACOS_CERTIFICATE} characters)"
fi

if [[ -z "${MACOS_CERTIFICATE_PASSWORD:-}" ]]; then
    echo "❌ MACOS_CERTIFICATE_PASSWORD is not set"
    exit 1
else
    echo "✅ MACOS_CERTIFICATE_PASSWORD is set"
fi

# Create temporary files
TEMP_P12="/tmp/debug-certificate.p12"
TEMP_PEM="/tmp/debug-certificate.pem"

echo ""
echo "🔐 Decoding and examining certificate..."

# Decode the certificate
echo "$MACOS_CERTIFICATE" | base64 --decode > "$TEMP_P12"
echo "✅ Certificate decoded successfully"

# Extract certificate information
echo ""
echo "📜 Certificate details:"
openssl pkcs12 -in "$TEMP_P12" -nokeys -out "$TEMP_PEM" -passin pass:"$MACOS_CERTIFICATE_PASSWORD" 2>/dev/null || {
    echo "❌ Failed to extract certificate from P12 file"
    echo "   This could indicate:"
    echo "   - Incorrect password"
    echo "   - Corrupted P12 file"
    echo "   - Invalid base64 encoding"
    rm -f "$TEMP_P12" "$TEMP_PEM"
    exit 1
}

# Show certificate subject and validity
openssl x509 -in "$TEMP_PEM" -noout -subject -dates -fingerprint 2>/dev/null || {
    echo "❌ Failed to read certificate details"
    rm -f "$TEMP_P12" "$TEMP_PEM"
    exit 1
}

echo ""
echo "🏢 Certificate identity information:"
CERT_SUBJECT=$(openssl x509 -in "$TEMP_PEM" -noout -subject 2>/dev/null | sed 's/subject=//')
echo "Subject: $CERT_SUBJECT"

# Check if it's a Developer ID certificate
if echo "$CERT_SUBJECT" | grep -q "Developer ID Application"; then
    echo "✅ This is a Developer ID Application certificate"

    # Extract the team identifier
    if echo "$CERT_SUBJECT" | grep -q "([A-Z0-9]\{10\})"; then
        TEAM_ID=$(echo "$CERT_SUBJECT" | grep -o '([A-Z0-9]\{10\})' | tr -d '()')
        echo "🆔 Team ID found in certificate: $TEAM_ID"

        # Check if it matches the script's expected team ID
        EXPECTED_TEAM="RVPAX6PXC6"
        if [[ "$TEAM_ID" == "$EXPECTED_TEAM" ]]; then
            echo "✅ Team ID matches expected value: $EXPECTED_TEAM"
        else
            echo "⚠️  Team ID mismatch:"
            echo "   Certificate: $TEAM_ID"
            echo "   Expected:    $EXPECTED_TEAM"
            echo "   You may need to update the IDENTITY variable in bundle-mac script"
        fi
    else
        echo "⚠️  Could not extract Team ID from certificate subject"
    fi
else
    echo "⚠️  This doesn't appear to be a Developer ID Application certificate"
    echo "   Expected: 'Developer ID Application: Company Name (TEAMID)'"
fi

echo ""
echo "⏰ Certificate validity:"
NOT_BEFORE=$(openssl x509 -in "$TEMP_PEM" -noout -startdate 2>/dev/null | cut -d= -f2)
NOT_AFTER=$(openssl x509 -in "$TEMP_PEM" -noout -enddate 2>/dev/null | cut -d= -f2)
echo "Valid from: $NOT_BEFORE"
echo "Valid to:   $NOT_AFTER"

# Check if certificate is expired
if openssl x509 -in "$TEMP_PEM" -checkend 0 -noout 2>/dev/null; then
    echo "✅ Certificate is currently valid"
else
    echo "❌ Certificate has expired!"
fi

echo ""
echo "🔗 Checking certificate chain..."

# Extract all certificates from P12
openssl pkcs12 -in "$TEMP_P12" -nokeys -cacerts -out "/tmp/debug-ca-certs.pem" -passin pass:"$MACOS_CERTIFICATE_PASSWORD" 2>/dev/null || true

if [[ -s "/tmp/debug-ca-certs.pem" ]]; then
    echo "✅ Certificate chain found in P12 file"
    CERT_COUNT=$(grep -c "BEGIN CERTIFICATE" "/tmp/debug-ca-certs.pem" 2>/dev/null || echo "0")
    echo "   Number of CA certificates: $CERT_COUNT"
else
    echo "⚠️  No certificate chain found in P12 file"
    echo "   This might require downloading intermediate certificates"
fi

echo ""
echo "🔑 Checking private key..."
if openssl pkcs12 -in "$TEMP_P12" -nocerts -nodes -out "/tmp/debug-private-key.pem" -passin pass:"$MACOS_CERTIFICATE_PASSWORD" 2>/dev/null; then
    if [[ -s "/tmp/debug-private-key.pem" ]] && grep -q "BEGIN PRIVATE KEY\|BEGIN RSA PRIVATE KEY" "/tmp/debug-private-key.pem"; then
        echo "✅ Private key found and appears valid"
    else
        echo "❌ Private key not found or invalid"
    fi
    rm -f "/tmp/debug-private-key.pem"
else
    echo "❌ Failed to extract private key"
fi

echo ""
echo "🛠️  Testing keychain operations..."

# Test keychain creation
TEST_KEYCHAIN="debug-test.keychain"
security delete-keychain "$TEST_KEYCHAIN" 2>/dev/null || true
security create-keychain -p "test123" "$TEST_KEYCHAIN" 2>/dev/null && {
    echo "✅ Keychain creation works"

    # Test certificate import
    if security import "$TEMP_P12" -k "$TEST_KEYCHAIN" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign 2>/dev/null; then
        echo "✅ Certificate import works"

        # Check if identity can be found
        if security find-identity -v -p codesigning "$TEST_KEYCHAIN" 2>/dev/null | grep -q "Developer ID Application"; then
            echo "✅ Certificate identity can be found in keychain"
        else
            echo "❌ Certificate identity not found in keychain after import"
        fi
    else
        echo "❌ Certificate import failed"
    fi

    # Cleanup
    security delete-keychain "$TEST_KEYCHAIN" 2>/dev/null || true
} || {
    echo "❌ Keychain creation failed"
}

echo ""
echo "📱 Apple Developer Portal recommendations:"
echo "1. Verify your certificate was issued for 'Developer ID Application' (not 'Mac Development')"
echo "2. Ensure the certificate was downloaded as a .p12 file with private key"
echo "3. Check that your Apple Developer account has the necessary permissions"
echo "4. Verify the certificate hasn't been revoked in your Apple Developer account"

# Cleanup
rm -f "$TEMP_P12" "$TEMP_PEM" "/tmp/debug-ca-certs.pem"

echo ""
echo "🎯 Next steps based on findings above:"
echo "- If Team ID mismatch: Update IDENTITY in bundle-mac script"
echo "- If certificate expired: Generate new certificate from Apple Developer Portal"
echo "- If no certificate chain: The simplified approach should work"
echo "- If import fails: Check certificate format and password"

echo ""
echo "✅ Certificate debugging complete!"
