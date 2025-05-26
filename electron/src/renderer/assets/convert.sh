# brew install imagemagick
#!/bin/bash
# Check if a PNG file was passed as an argument
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <png-file>"
    exit 1
fi

# PNG file passed as a parameter
PNG_FILE=$1

# Check if the file exists
if [ ! -f "$PNG_FILE" ]; then
    echo "File not found: $PNG_FILE"
    exit 1
fi

# Get the base name of the file (without extension)
BASE_NAME=$(basename "$PNG_FILE" .png)

# Create a folder for the iconset
ICONSET_FOLDER="${BASE_NAME}.iconset"
mkdir -p "$ICONSET_FOLDER"

# Convert PNG to different sizes using magick instead of convert
magick "$PNG_FILE" -resize 16x16 "$ICONSET_FOLDER/icon_16x16.png"
magick "$PNG_FILE" -resize 32x32 "$ICONSET_FOLDER/icon_32x32.png"
magick "$PNG_FILE" -resize 64x64 "$ICONSET_FOLDER/icon_64x64.png"
magick "$PNG_FILE" -resize 128x128 "$ICONSET_FOLDER/icon_128x128.png"
magick "$PNG_FILE" -resize 256x256 "$ICONSET_FOLDER/icon_256x256.png"
magick "$PNG_FILE" -resize 512x512 "$ICONSET_FOLDER/icon_512x512.png"

# Create Retina (2x) sizes from the original 512x512 PNG
magick "$PNG_FILE" -resize 32x32 "$ICONSET_FOLDER/icon_16x16@2x.png"
magick "$PNG_FILE" -resize 64x64 "$ICONSET_FOLDER/icon_32x32@2x.png"
magick "$PNG_FILE" -resize 128x128 "$ICONSET_FOLDER/icon_64x64@2x.png"
magick "$PNG_FILE" -resize 256x256 "$ICONSET_FOLDER/icon_128x128@2x.png"
magick "$PNG_FILE" -resize 512x512 "$ICONSET_FOLDER/icon_256x256@2x.png"

# Convert the iconset folder to .icns using iconutil
iconutil -c icns "$ICONSET_FOLDER"

# Move the generated .icns file to the current directory
mv "${BASE_NAME}.icns" ./

# Clean up: remove the iconset folder
rm -rf "$ICONSET_FOLDER"

echo "Conversion complete: ${BASE_NAME}.icns created for mac."


magick "$PNG_FILE" -define icon:auto-resize=256,128,96,64,48,32,16 eltor-logo.ico

echo "Conversion complete: ${BASE_NAME}.ico created for windows."
