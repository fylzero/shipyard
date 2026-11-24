"""Fit a squircle tile to Apple's macOS icon grid: an 824px tile inset 100px on
a 1024px canvas, with a faint drop shadow. This made app-icon-macos.png, the
Dock and Command-Tab icon. Needs Pillow, NumPy and SciPy.

The artwork's own tile is a rounded square with a light border ring, and its
corners don't follow Apple's squircle curve, so macOS showed slivers of that
ring along the top and bottom edges. This drops the ring: it keeps only the
art inside the border, grows that inner background out to the edge, and cuts
it with Apple's squircle (a superellipse, n = 5), so the tile ends cleanly.

The input is either the original art on a black background or an existing
app-icon-macos.png (transparent around the tile); running it again on its own
output is safe.

Usage: fit-squircle.py squircle-art.png app-icon-macos.png [border_px]
  border_px: how far in from the tile's edge the border ring ends, measured
  on the 824px tile (default 20)."""
import sys
import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

src, dst = sys.argv[1], sys.argv[2]
BORDER = int(sys.argv[3]) if len(sys.argv) > 3 else 20
SIZE, INSET = 1024, 100
TILE = SIZE - 2 * INSET
SS = 4  # supersampling for the squircle mask edge

img = Image.open(src).convert("RGBA")
a = np.asarray(img).astype(int)
if a[..., 3].min() < 128:
    tile = a[..., 3] >= 128  # already cut out
else:
    dark = a[..., :3].max(2) <= 10
    # Background = dark pixels connected to the image border.
    labels, _ = ndimage.label(dark)
    edge = np.unique(np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]]))
    tile = ~np.isin(labels, edge[edge > 0])
tile = ndimage.binary_fill_holes(tile)
ys, xs = np.where(tile)
box = (xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)

# Crop to the tile and scale it to the 824px grid.
rgb = img.convert("RGB").crop(box).resize((TILE, TILE), Image.LANCZOS)
tmask = Image.fromarray((tile * 255).astype(np.uint8)).crop(box).resize((TILE, TILE), Image.LANCZOS)
inner = ndimage.binary_erosion(np.asarray(tmask) >= 128, iterations=BORDER)

# Fill everything outside the inner area with its nearest inner pixel, so the
# tile's background runs smoothly out to the new edge.
_, (iy, ix) = ndimage.distance_transform_edt(~inner, return_indices=True)
px = np.asarray(rgb)[iy, ix]
# Soften the seam a little where the fill meets the kept art.
blurred = np.asarray(Image.fromarray(px).filter(ImageFilter.GaussianBlur(3)))
dist = ndimage.distance_transform_edt(~inner)
w = np.clip(dist / 6, 0, 1)[..., None]
px = (px * (1 - w) + blurred * w).astype(np.uint8)

# Apple's squircle.
n = 5.0
g = (np.arange(TILE * SS) + 0.5) / (TILE * SS) * 2 - 1
inside = (np.abs(g)[None, :] ** n + np.abs(g)[:, None] ** n) <= 1
sq = Image.fromarray((inside * 255).astype(np.uint8)).resize((TILE, TILE), Image.BOX)

rgba = Image.fromarray(px).convert("RGBA")
rgba.putalpha(sq)

canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
alpha = Image.new("L", (SIZE, SIZE), 0)
alpha.paste(sq, (INSET, INSET))
shadow = alpha.filter(ImageFilter.GaussianBlur(10)).point(lambda v: int(v * 0.22))
canvas.paste(Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 255)), (0, 6), shadow)
layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
layer.paste(rgba, (INSET, INSET))
canvas = Image.alpha_composite(canvas, layer)
canvas.save(dst)
print(f"{src}: tile box {box} -> {dst}")
