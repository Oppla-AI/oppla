# DMG Installer Assets

This directory contains assets for creating branded DMG installers for Oppla AI IDE.

## Files

- `background.png` - Background image for the DMG window (800x400px)
- `volume-icon.icns` - Icon for the DMG volume (shows in Finder sidebar)
- `create_background.sh` - Script to generate the background image

## Creating/Updating Assets

### Background Image
The background image should be 800x400 pixels with Oppla branding. To create or update:

```bash
./create_background.sh
```

Or create a custom image with:
- Dark theme matching Oppla's design
- Clear instructions for installation
- Oppla logo/branding
- Arrow indicating drag direction

### Volume Icon
The volume icon is automatically generated from the app icon during the build process.
To manually create:

```bash
# Convert PNG to ICNS
sips -s format icns ../images/app-icon.png --out volume-icon.icns
```

## DMG Layout

The DMG window shows:
- Oppla.app on the left (position: 200, 190)
- Applications folder alias on the right (position: 600, 185)
- Custom background with installation instructions
- Window size: 800x400 pixels

## Build Process

The branded DMG is created automatically during the build process using the `script/bundle-mac` script, which calls `script/create-branded-dmg.sh` to apply these assets.