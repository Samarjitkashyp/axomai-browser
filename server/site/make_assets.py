"""Makes the landing page's images from the browser's own artwork (run once; the results are committed).
   python make_assets.py   (needs Pillow)"""
import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, '..', '..', 'ui', 'assets', 'images')
OUT = os.path.join(HERE, 'assets')
os.makedirs(OUT, exist_ok=True)

def rounded_logo(size):
    """The logo is a rounded square on a black background; cut the corners out so it sits on any colour."""
    im = Image.open(os.path.join(SRC, 'axomai_logo.png')).convert('RGBA')
    w, h = im.size
    box = (int(w * 0.055), int(h * 0.055), int(w * 0.945), int(h * 0.945))
    im = im.crop(box)
    w, h = im.size
    mask = Image.new('L', (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius=int(w * 0.17), fill=255)
    im.putalpha(mask)
    return im.resize((size, size), Image.LANCZOS)

for name, size in (('logo-512.png', 512), ('logo-192.png', 192), ('apple-touch-icon.png', 180), ('favicon-48.png', 48), ('favicon-32.png', 32)):
    img = rounded_logo(size)
    if name == 'apple-touch-icon.png':  # iOS does not like transparency
        bg = Image.new('RGBA', (size, size), (255, 255, 255, 255))
        bg.alpha_composite(img)
        img = bg
    img.save(os.path.join(OUT, name), optimize=True)
rounded_logo(64).save(os.path.join(HERE, 'favicon.ico'), sizes=[(16, 16), (32, 32), (48, 48)])

def photo(src, dst, width=1000, quality=76):
    im = Image.open(os.path.join(SRC, src)).convert('RGB')
    h = int(im.height * width / im.width)
    im.resize((width, h), Image.LANCZOS).save(os.path.join(OUT, dst), 'JPEG', quality=quality, optimize=True, progressive=True)

photo('tea_garden_bg.jpg', 'tea-garden.jpg', 1400, 78)
photo('kaziranga.jpg', 'kaziranga.jpg')
photo('bihu.jpg', 'bihu.jpg')
photo('rhino.jpg', 'rhino.jpg')

# social preview, 1200x630
W, H = 1200, 630
bg = Image.open(os.path.join(SRC, 'tea_garden_bg.jpg')).convert('RGB')
r = max(W / bg.width, H / bg.height)
bg = bg.resize((int(bg.width * r), int(bg.height * r)), Image.LANCZOS)
bg = bg.crop(((bg.width - W) // 2, (bg.height - H) // 2, (bg.width - W) // 2 + W, (bg.height - H) // 2 + H))
shade = Image.new('RGBA', (W, H), (2, 44, 34, 0))
px = shade.load()
for x in range(W):
    a = int(235 - 140 * (x / W))
    for y in range(H):
        px[x, y] = (2, 44, 34, a)
card = bg.convert('RGBA')
card.alpha_composite(shade)
d = ImageDraw.Draw(card)
fonts = r'C:\Windows\Fonts'
def font(name, size):
    try:
        return ImageFont.truetype(os.path.join(fonts, name), size)
    except OSError:
        return ImageFont.load_default()
d.text((70, 150), 'Axomai Browser', font=font('segoeuib.ttf', 84), fill=(255, 255, 255, 255))
d.text((74, 262), 'The free AI browser made in Assam', font=font('segoeui.ttf', 44), fill=(167, 243, 208, 255))
d.text((74, 340), 'Built-in AI chat  \u2022  Assamese, Hindi, Bengali  \u2022  Private by default', font=font('segoeui.ttf', 30), fill=(236, 253, 245, 255))
d.rounded_rectangle((74, 440, 520, 520), radius=40, fill=(16, 185, 129, 255))
d.text((126, 458), 'Download for Windows', font=font('segoeuib.ttf', 34), fill=(255, 255, 255, 255))
card.alpha_composite(rounded_logo(170), (W - 250, 70))
card.convert('RGB').save(os.path.join(OUT, 'og-image.jpg'), 'JPEG', quality=88, optimize=True)
print('assets ready:', sorted(os.listdir(OUT)))
