"""Fit a squircle tile to Apple's macOS icon grid (an 824px tile inset 100px on
a 1024px canvas) with a faint drop shadow. This made app-icon-macos.png, the
Dock and Command-Tab icon. Needs Pillow, NumPy and SciPy.

How macOS 26 and later treat a legacy .icns icon decides the shape here:
  - If the opaque tile reads as Apple's rounded square, macOS re-cuts it with
    its own mask and draws its own glass edge on top.
  - If it doesn't (a superellipse curve fails this at the larger sizes, as did
    the artwork's own tile), macOS shrinks the icon into a gray squircle, and
    the edges of that container and the shrunken tile show as thin lines.
So the tile is a plain rounded rectangle (corner radius 185 on the 824 grid),
made a few pixels larger than the grid (BLEED) so the system's mask always lands
on solid art, never on the anti-aliased edge or the shadow.

The artwork's own tile has a light rim that wouldn't line up with the system's
edge, so this drops it: it keeps only the art inside the rim and grows that
inner background out to the new edge.

The input is the art on a flat background (any colour, sampled from the
corners) or an existing app-icon-macos.png (transparent around the tile).

Usage: fit-squircle.py squircle-art.png app-icon-macos.png [border_px]
  border_px: how far in from the tile's edge the art's own rim ends, measured
             on the 824px tile (default 28)"""
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy import ndimage

src, dst = sys.argv[1], sys.argv[2]
BORDER = int(sys.argv[3]) if len(sys.argv) > 3 else 28
SIZE, GRID_INSET, BLEED = 1024, 100, 4
INSET = GRID_INSET - BLEED
TILE = SIZE - 2 * INSET
RADIUS = 185 * TILE / 824
SS = 4  # supersampling for the mask edge

img = Image.open(src).convert("RGBA")
a = np.asarray(img).astype(int)
if a[..., 3].min() < 128:
    tile = a[..., 3] >= 128  # already cut out
else:
    c = np.median(np.array([a[0, 0], a[0, -1], a[-1, 0], a[-1, -1]]), axis=0)[:3]
    near = np.abs(a[..., :3] - c).max(2) <= 12
    # Background = background-coloured pixels connected to the image border.
    labels, _ = ndimage.label(near)
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
dist, (iy, ix) = ndimage.distance_transform_edt(~inner, return_indices=True)
px = np.asarray(rgb)[iy, ix].astype(float)
# Soften the streaks the fill leaves, more the further out it reaches.
blurred = np.asarray(Image.fromarray(px.astype(np.uint8)).filter(ImageFilter.GaussianBlur(6))).astype(float)
w = np.clip(dist / 8, 0, 1)[..., None]
px = px * (1 - w) + blurred * w

# Apple's rounded square, supersampled.
big = Image.new("L", (TILE * SS, TILE * SS), 0)
ImageDraw.Draw(big).rounded_rectangle((0, 0, TILE * SS - 1, TILE * SS - 1), radius=RADIUS * SS, fill=255)
sq = big.resize((TILE, TILE), Image.BOX)

rgba = Image.fromarray(np.clip(px, 0, 255).astype(np.uint8)).convert("RGBA")
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
