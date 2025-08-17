#!/usr/bin/env python3

"""
Create a branded background image for Oppla DMG installer
"""

import sys

try:
    from PIL import Image, ImageDraw, ImageFont
    import os
except ImportError:
    print("PIL (Pillow) is required. Install with: pip3 install Pillow")
    sys.exit(1)

# Constants
WIDTH = 800
HEIGHT = 400
BACKGROUND_COLOR = (26, 26, 26)  # Dark background #1a1a1a
GRADIENT_END = (45, 45, 45)  # Slightly lighter #2d2d2d
TEXT_COLOR = (255, 255, 255)  # White
SUBTITLE_COLOR = (136, 136, 136)  # Gray #888888
URL_COLOR = (102, 102, 102)  # Darker gray #666666
ARROW_COLOR = (85, 85, 85)  # Dark gray #555555

def create_gradient(width, height, start_color, end_color):
    """Create a vertical gradient"""
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)
    
    for y in range(height):
        # Calculate interpolated color
        ratio = y / height
        r = int(start_color[0] * (1 - ratio) + end_color[0] * ratio)
        g = int(start_color[1] * (1 - ratio) + end_color[1] * ratio)
        b = int(start_color[2] * (1 - ratio) + end_color[2] * ratio)
        draw.rectangle([(0, y), (width, y + 1)], fill=(r, g, b))
    
    return img

def draw_arrow(draw, start_x, start_y, end_x, end_y, color, width=3):
    """Draw an arrow from start to end"""
    # Draw the line
    draw.line([(start_x, start_y), (end_x, start_y)], fill=color, width=width)
    
    # Draw arrowhead
    arrow_size = 15
    draw.polygon([
        (end_x - arrow_size, start_y - arrow_size//2),
        (end_x - arrow_size, start_y + arrow_size//2),
        (end_x, start_y)
    ], fill=color)

def create_dmg_background():
    """Create the DMG background image"""
    
    # Create gradient background
    img = create_gradient(WIDTH, HEIGHT, BACKGROUND_COLOR, GRADIENT_END)
    draw = ImageDraw.Draw(img)
    
    # Try to use system fonts, fall back to default if not available
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 28)
        subtitle_font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 18)
        url_font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    except:
        # Use default font if system fonts aren't available
        title_font = ImageFont.load_default()
        subtitle_font = ImageFont.load_default()
        url_font = ImageFont.load_default()
    
    # Draw title
    title = "Oppla AI IDE"
    title_bbox = draw.textbbox((0, 0), title, font=title_font)
    title_width = title_bbox[2] - title_bbox[0]
    title_x = (WIDTH - title_width) // 2
    draw.text((title_x, 40), title, fill=TEXT_COLOR, font=title_font)
    
    # Draw installation instruction
    instruction = "Drag Oppla to Applications folder to install"
    instruction_bbox = draw.textbbox((0, 0), instruction, font=subtitle_font)
    instruction_width = instruction_bbox[2] - instruction_bbox[0]
    instruction_x = (WIDTH - instruction_width) // 2
    draw.text((instruction_x, HEIGHT // 2 + 100), instruction, fill=SUBTITLE_COLOR, font=subtitle_font)
    
    # Draw arrow
    draw_arrow(draw, 280, HEIGHT // 2, 520, HEIGHT // 2, ARROW_COLOR, width=3)
    
    # Draw URL at bottom
    url = "oppla.ai"
    url_bbox = draw.textbbox((0, 0), url, font=url_font)
    url_width = url_bbox[2] - url_bbox[0]
    url_x = (WIDTH - url_width) // 2
    draw.text((url_x, HEIGHT - 40), url, fill=URL_COLOR, font=url_font)
    
    # Add subtle visual elements - app icon placeholder on left
    icon_x, icon_y = 200, 160
    icon_size = 80
    draw.ellipse([icon_x - icon_size//2, icon_y - icon_size//2, 
                  icon_x + icon_size//2, icon_y + icon_size//2], 
                 outline=ARROW_COLOR, width=2)
    draw.text((icon_x - 25, icon_y - 10), "Oppla", fill=SUBTITLE_COLOR, font=url_font)
    
    # Applications folder icon placeholder on right
    folder_x, folder_y = 600, 160
    draw.rectangle([folder_x - 40, folder_y - 30, folder_x + 40, folder_y + 30], 
                   outline=ARROW_COLOR, width=2)
    draw.text((folder_x - 45, folder_y - 5), "Applications", fill=SUBTITLE_COLOR, font=url_font)
    
    # Save the image
    output_path = "background.png"
    img.save(output_path, "PNG")
    print(f"✅ Background image created: {output_path}")
    print(f"   Dimensions: {WIDTH}x{HEIGHT}")
    
    # Also create a 2x version for retina displays
    img_2x = img.resize((WIDTH * 2, HEIGHT * 2), Image.Resampling.LANCZOS)
    img_2x.save("background@2x.png", "PNG")
    print(f"✅ Retina background created: background@2x.png")

if __name__ == "__main__":
    create_dmg_background()