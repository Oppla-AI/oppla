# DMG Background Design Guide

## Background Image Specifications

The `background.png` should be:
- **Dimensions**: 800x400 pixels
- **Format**: PNG with transparency support (if needed)
- **Color Scheme**: Dark theme matching Oppla's branding
  - Primary background: #1a1a1a
  - Secondary/gradient: #2d2d2d
  - Text: White (#ffffff) for titles, Gray (#888888) for instructions
  - Accents: Oppla brand colors

## Layout Guidelines

The DMG window layout positions items as follows:

```
+--------------------------------------------------+
|                  Oppla AI IDE                   |  <- Title (y: 40px)
|                                                  |
|    [Oppla.app]  ------>  [Applications]        |  <- Icons (y: 160-190px)
|     (200,190)              (600,185)            |
|                                                  |
|   Drag Oppla to Applications folder to install  |  <- Instructions (y: 280px)
|                                                  |
|                   oppla.ai                      |  <- URL (y: 370px)
+--------------------------------------------------+
```

## Design Elements to Include

1. **Title**: "Oppla AI IDE" - prominent at top
2. **Visual Arrow**: Indicating drag direction from app to Applications
3. **Installation Text**: Clear instructions
4. **Branding**: Oppla logo or watermark (subtle)
5. **URL**: oppla.ai website

## Creating a Custom Background

### Using Design Tools

1. **Figma/Sketch**: Create a 800x400 artboard with the layout above
2. **Photoshop**: New document at 800x400px, 72 DPI
3. **Affinity Designer**: Similar setup

### Color Palette

```css
:root {
  --bg-primary: #1a1a1a;
  --bg-secondary: #2d2d2d;
  --text-primary: #ffffff;
  --text-secondary: #888888;
  --text-muted: #666666;
  --accent: #555555;
}
```

### Export Settings

- Format: PNG-24
- Transparency: Preserved (if using)
- Color Profile: sRGB
- Optimization: Compressed

## Testing

After creating a custom background:

1. Replace `background.png` in this directory
2. Build a test DMG: `script/bundle-mac -l`
3. Mount the DMG and verify appearance
4. Check on both Light and Dark macOS themes

## Retina Support

For high-resolution displays, also provide:
- `background@2x.png` at 1600x800 pixels

The DMG creation script will automatically use the @2x version on Retina displays.