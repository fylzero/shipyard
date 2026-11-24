"""Cut a squircle tile off its black background and place it on Apple's
macOS icon grid (an 824px tile inset 100px on a 1024px canvas, with a faint
drop shadow). This made app-icon-macos.png, the Dock and Command-Tab icon.
Needs Pillow, NumPy and SciPy.

Usage: fit-squircle.py squircle-art.png app-icon-macos.png"""
import sys
import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

src, dst = sys.argv[1], sys.argv[2]
SIZE, INSET = 1024, 100
TILE = SIZE - 2 * INSET

img = Image.open(src).convert("RGB")
a = np.asarray(img).astype(int)
dark = a.max(2) <= 10
# Background = dark pixels connected to the image border.
labels, _ = ndimage.label(dark)
edge = np.unique(np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]]))
bg = np.isin(labels, edge[edge > 0])
tile = ndimage.binary_fill_holes(~bg)
# Pull the edge in a couple of pixels to drop the anti-aliased black fringe.
tile = ndimage.binary_erosion(tile, iterations=3)
ys, xs = np.where(tile)
box = (xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)
mask = Image.fromarray((tile * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2))
rgba = img.convert("RGBA")
rgba.putalpha(mask)
rgba = rgba.crop(box).resize((TILE, TILE), Image.LANCZOS)

canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
alpha = Image.new("L", (SIZE, SIZE), 0)
alpha.paste(rgba.getchannel("A"), (INSET, INSET))
shadow = alpha.filter(ImageFilter.GaussianBlur(10)).point(lambda v: int(v * 0.22))
canvas.paste(Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 255)), (0, 6), shadow)
layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
layer.paste(rgba, (INSET, INSET))
canvas = Image.alpha_composite(canvas, layer)
canvas.save(dst)
print(f"{src}: tile box {box} -> {dst}")
