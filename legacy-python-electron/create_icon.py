from PIL import Image, ImageDraw, ImageFont
import os

def create_icon():
    size = (256, 256)
    img = Image.new('RGBA', size, color=(0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    
    # Draw a rounded rectangle (background)
    d.rounded_rectangle([(10, 10), (246, 246)], radius=40, fill=(65, 105, 225), outline=(255, 255, 255), width=5)
    
    # Draw text "C"
    try:
        # Try to use a default font
        font = ImageFont.truetype("arial.ttf", 150)
    except IOError:
        font = ImageFont.load_default()
    
    # Draw "C" in center (approximate centering)
    d.text((128, 128), "C", fill=(255, 255, 255), font=font, anchor="mm")
    
    # Ensure build directory exists
    os.makedirs('desktop/build', exist_ok=True)
    
    # Save as ICO
    img.save('desktop/build/icon.ico', format='ICO', sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])
    print("Icon created at desktop/build/icon.ico")

if __name__ == "__main__":
    try:
        create_icon()
    except ImportError:
        print("Pillow not installed, installing...")
        import subprocess
        subprocess.check_call(["pip", "install", "Pillow"])
        create_icon()
