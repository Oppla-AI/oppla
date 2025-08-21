#!/usr/bin/env bash

set -euo pipefail

echo "Fixing dpkg corruption..."

# Check if we need sudo
if [ "$(id -u)" -eq 0 ]; then
    maysudo=''
else
    maysudo="$(command -v sudo || command -v doas || true)"
fi

# Backup current corrupted status file
if [ -f /var/lib/dpkg/status ]; then
    echo "Backing up corrupted status file..."
    $maysudo mv /var/lib/dpkg/status /var/lib/dpkg/status.corrupted.$(date +%Y%m%d_%H%M%S)
fi

# Try to restore from automatic backup
if [ -f /var/lib/dpkg/status-old ]; then
    echo "Restoring from status-old backup..."
    $maysudo cp /var/lib/dpkg/status-old /var/lib/dpkg/status
else
    echo "No backup found, creating empty status file..."
    $maysudo touch /var/lib/dpkg/status
fi

# Try to fix dpkg database
echo "Configuring dpkg..."
$maysudo dpkg --configure -a || true

# Clear apt lists and update
echo "Clearing apt lists..."
$maysudo rm -rf /var/lib/apt/lists/*
$maysudo apt-get clean

echo "Updating package lists..."
$maysudo apt-get update

# Fix any broken dependencies
echo "Fixing broken dependencies..."
$maysudo apt-get install -f -y

echo "dpkg corruption fix completed!"