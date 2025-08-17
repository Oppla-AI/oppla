#!/usr/bin/env bash

# Create a simple background image for DMG installer
# This creates a dark-themed background with Oppla branding

OUTPUT="background.png"
WIDTH=800
HEIGHT=400

# Check if ImageMagick is installed
if ! command -v convert &> /dev/null; then
    echo "ImageMagick is required but not installed."
    echo "Install with: brew install imagemagick"
    exit 1
fi

echo "Creating DMG background image..."

# Create the background with gradient and text
convert -size ${WIDTH}x${HEIGHT} \
    xc:'#1a1a1a' \
    -fill 'gradient:#1a1a1a-#2d2d2d' \
    -draw "rectangle 0,0 ${WIDTH},${HEIGHT}" \
    -fill white \
    -font Helvetica-Bold \
    -pointsize 28 \
    -gravity North \
    -annotate +0+40 'Oppla AI IDE' \
    -fill '#888888' \
    -font Helvetica \
    -pointsize 18 \
    -gravity Center \
    -annotate +0+120 'Drag Oppla to Applications folder to install' \
    -fill '#666666' \
    -pointsize 14 \
    -gravity South \
    -annotate +0+30 'oppla.ai' \
    "$OUTPUT"

# Add arrow graphic (simple line with arrowhead)
convert "$OUTPUT" \
    -stroke '#555555' \
    -strokewidth 3 \
    -fill none \
    -draw "path 'M 300,200 L 500,200'" \
    -stroke '#555555' \
    -fill '#555555' \
    -draw "polygon 490,195 490,205 505,200" \
    "$OUTPUT"

echo "Background image created: $OUTPUT"
echo "Dimensions: ${WIDTH}x${HEIGHT}"