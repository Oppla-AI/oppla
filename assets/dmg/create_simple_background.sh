#!/usr/bin/env bash

# Create a simple branded background for Oppla DMG using macOS built-in tools
# This creates a dark background with text overlays

OUTPUT="background.png"
WIDTH=800
HEIGHT=400

echo "Creating simple DMG background using macOS tools..."

# Create a dark background using sips and Core Image
# First create a solid color image
cat > temp_background.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
<style>
body {
    margin: 0;
    padding: 0;
    width: 800px;
    height: 400px;
    background: linear-gradient(to bottom, #1a1a1a, #2d2d2d);
    font-family: -apple-system, Helvetica, Arial, sans-serif;
    color: white;
    position: relative;
    overflow: hidden;
}
.title {
    text-align: center;
    font-size: 28px;
    font-weight: bold;
    margin-top: 40px;
}
.instruction {
    text-align: center;
    font-size: 18px;
    color: #888;
    position: absolute;
    bottom: 120px;
    width: 100%;
}
.url {
    text-align: center;
    font-size: 14px;
    color: #666;
    position: absolute;
    bottom: 30px;
    width: 100%;
}
.app-icon {
    position: absolute;
    left: 160px;
    top: 120px;
    width: 80px;
    height: 80px;
    border: 2px solid #555;
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #888;
    font-size: 12px;
}
.folder-icon {
    position: absolute;
    right: 160px;
    top: 120px;
    width: 80px;
    height: 80px;
    border: 2px solid #555;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #888;
    font-size: 11px;
    text-align: center;
}
.arrow {
    position: absolute;
    left: 280px;
    top: 160px;
    width: 240px;
    height: 2px;
    background: #555;
}
.arrow::after {
    content: '';
    position: absolute;
    right: -10px;
    top: -5px;
    width: 0;
    height: 0;
    border-left: 12px solid #555;
    border-top: 6px solid transparent;
    border-bottom: 6px solid transparent;
}
</style>
</head>
<body>
    <div class="title">Oppla AI IDE</div>
    <div class="app-icon">Oppla</div>
    <div class="folder-icon">Applications</div>
    <div class="arrow"></div>
    <div class="instruction">Drag Oppla to Applications folder to install</div>
    <div class="url">oppla.ai</div>
</body>
</html>
EOF

# Use webkit2png if available, otherwise try screencapture
if command -v webkit2png &> /dev/null; then
    webkit2png --width=800 --height=400 --filename=background temp_background.html
    mv background-full.png background.png
    rm -f background-thumb.png background-clipped.png
elif command -v wkhtmltoimage &> /dev/null; then
    wkhtmltoimage --width 800 --height 400 temp_background.html background.png
else
    # Fallback: create a simple solid color image
    echo "Creating fallback solid color background..."
    # Create a 1x1 dark pixel and scale it up
    printf '\x1a\x1a\x1a' | xxd -r -p > temp_pixel.rgb
    # Convert to PNG using sips (this is a workaround)
    # First create a small PNG from screenshot
    osascript -e 'tell application "System Events" to set frontmost of process "Terminal" to true'
    sleep 0.5
    screencapture -R0,0,1,1 temp_small.png
    # Now use sips to resize and set color
    sips -z 400 800 temp_small.png --out background.png
    rm -f temp_pixel.rgb temp_small.png
fi

# Clean up
rm -f temp_background.html

if [ -f "background.png" ]; then
    echo "✅ Background image created: background.png"
    echo "   Dimensions: ${WIDTH}x${HEIGHT}"
    echo ""
    echo "Note: For a more sophisticated background with gradients and graphics,"
    echo "consider using a graphic design tool like Sketch, Figma, or Photoshop"
    echo "to create a custom 800x400px background.png"
else
    echo "❌ Failed to create background image"
    echo "Please create a custom 800x400px background.png manually"
fi