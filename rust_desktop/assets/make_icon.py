"""Makes the program icon (axomai.ico, icon-256.png) from icons/axomai_logo.png. Run once; the results are committed.
   python make_icon.py   (needs Pillow)"""
import os
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
src = Image.open(os.path.join(HERE, 'icons', 'axomai_logo.png')).convert('RGBA')

def rounded(size):
    """The logo is a rounded square on a black background; cut the corners out so the icon is transparent there."""
    w, h = src.size
    im = src.crop((int(w * 0.055), int(h * 0.055), int(w * 0.945), int(h * 0.945)))
    w, h = im.size
    mask = Image.new('L', (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius=int(w * 0.17), fill=255)
    im.putalpha(mask)
    return im.resize((size, size), Image.LANCZOS)

big = rounded(256)
big.save(os.path.join(HERE, 'icon-256.png'), optimize=True)
big.save(os.path.join(HERE, 'axomai.ico'), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print('icon ready', os.path.getsize(os.path.join(HERE, 'axomai.ico')), 'bytes')
