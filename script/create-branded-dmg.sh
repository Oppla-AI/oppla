#!/usr/bin/env bash

set -euo pipefail

# Function to create a branded DMG with custom background and layout
create_branded_dmg() {
    local dmg_source_directory="$1"
    local dmg_file_path="$2"
    local app_name="${3:-Oppla.app}"
    
    echo "Creating branded DMG installer..."
    
    # Check if create-dmg is installed
    if ! command -v create-dmg &> /dev/null; then
        echo "create-dmg not found. Installing..."
        if command -v brew &> /dev/null; then
            brew install create-dmg
        else
            echo "Installing create-dmg from source..."
            temp_dir=$(mktemp -d)
            git clone https://github.com/create-dmg/create-dmg.git "$temp_dir/create-dmg"
            ln -s "$temp_dir/create-dmg/create-dmg" /usr/local/bin/create-dmg
        fi
    fi
    
    # Prepare assets
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local assets_dir="$(dirname "$script_dir")/assets/dmg"
    
    # Create volume icon if it doesn't exist
    if [ ! -f "$assets_dir/volume-icon.icns" ]; then
        echo "Creating volume icon from app icon..."
        local app_icon="$(dirname "$script_dir")/assets/images/app-icon.png"
        if [ -f "$app_icon" ]; then
            # Create temporary iconset
            local iconset_dir=$(mktemp -d)/Oppla.iconset
            mkdir -p "$iconset_dir"
            
            # Use sips to create different sizes
            sips -z 16 16     "$app_icon" --out "$iconset_dir/icon_16x16.png"
            sips -z 32 32     "$app_icon" --out "$iconset_dir/icon_16x16@2x.png"
            sips -z 32 32     "$app_icon" --out "$iconset_dir/icon_32x32.png"
            sips -z 64 64     "$app_icon" --out "$iconset_dir/icon_32x32@2x.png"
            sips -z 128 128   "$app_icon" --out "$iconset_dir/icon_128x128.png"
            sips -z 256 256   "$app_icon" --out "$iconset_dir/icon_128x128@2x.png"
            sips -z 256 256   "$app_icon" --out "$iconset_dir/icon_256x256.png"
            sips -z 512 512   "$app_icon" --out "$iconset_dir/icon_256x256@2x.png"
            sips -z 512 512   "$app_icon" --out "$iconset_dir/icon_512x512.png"
            sips -z 1024 1024 "$app_icon" --out "$iconset_dir/icon_512x512@2x.png"
            
            # Convert to icns
            iconutil -c icns "$iconset_dir" -o "$assets_dir/volume-icon.icns"
            rm -rf "$(dirname "$iconset_dir")"
            echo "Volume icon created successfully"
        fi
    fi
    
    # Build the DMG with create-dmg
    local create_dmg_args=(
        --volname "Oppla"
        --window-pos 200 120
        --window-size 660 400
        --icon-size 160
        --text-size 14
        --icon "$app_name" 180 170
        --hide-extension "$app_name"
        --app-drop-link 480 170
        --format UDZO
        --no-internet-enable
        --hdiutil-quiet
    )
    
    # Add volume icon if it exists
    if [ -f "$assets_dir/volume-icon.icns" ]; then
        create_dmg_args+=(--volicon "$assets_dir/volume-icon.icns")
    fi
    
    # Create the DMG
    create-dmg "${create_dmg_args[@]}" \
        "$dmg_file_path" \
        "$dmg_source_directory" || {
            echo "Warning: create-dmg failed, falling back to hdiutil"
            # Fallback to basic hdiutil if create-dmg fails
            hdiutil create -volname Oppla -srcfolder "$dmg_source_directory" -ov -format UDZO "$dmg_file_path"
        }
    
    echo "Branded DMG created successfully at $dmg_file_path"
}

# If called directly with arguments
if [ $# -ge 2 ]; then
    create_branded_dmg "$@"
fi