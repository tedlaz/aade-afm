# /// script
# dependencies = ["pillow"]
# ///
"""Φτιάχνει icon.png / icon.ico: μπλε πλακίδιο, έγγραφο μητρώου και μεγεθυντικός φακός.
Χρήση: uv run assets/make_icon.py
"""
from pathlib import Path
from PIL import Image, ImageDraw

S = 1024  # σχεδίαση σε μεγάλο μέγεθος για ομαλές άκρες
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

d.rounded_rectangle((40, 40, S - 40, S - 40), radius=200, fill=(31, 58, 95))
# έγγραφο
d.rounded_rectangle((230, 180, 690, 800), radius=50, fill=(255, 255, 255))
for i, w in enumerate((330, 260, 330, 200)):
    y = 300 + i * 110
    d.rounded_rectangle((310, y, 310 + w, y + 40), radius=20, fill=(160, 178, 200))
# φακός
cx, cy, r = 650, 640, 165
d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=(255, 255, 255), outline=(242, 153, 0), width=60)
d.line((cx + 115, cy + 115, cx + 270, cy + 270), fill=(242, 153, 0), width=95)
d.ellipse((cx + 222, cy + 222, cx + 318, cy + 318), fill=(242, 153, 0))

out = Path(__file__).parent
big = img.resize((256, 256), Image.LANCZOS)
big.save(out / "icon.png")
big.save(out / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("ok")
