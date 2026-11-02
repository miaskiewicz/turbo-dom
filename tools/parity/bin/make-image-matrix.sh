#!/bin/sh
# Generate the pinned image matrix used by the image-dedupe gates, plus HTML
# pages that embed each image (data: URIs) for the html2pdf and surf corpora.
# Usage: make-image-matrix.sh OUT_DIR
set -eu
out=$1
img="$out/images"
mkdir -p "$img" "$out/pages"
src="$img/src.png"
# Deterministic source: gradient + text-free shapes, 64x40.
magick -size 64x40 gradient:'#d02020-#2040c0' -fill '#20c040' -draw 'circle 32,20 32,8' \
  -define png:exclude-chunks=date,time "$src"
q() { magick "$src" "$@"; }
q -define png:exclude-chunks=date,time -strip -quality 90 "$img/jpeg-baseline.jpg"
q -strip -interlace JPEG -quality 90 "$img/jpeg-progressive.jpg"
q -strip -colorspace Gray -quality 90 "$img/jpeg-gray.jpg"
q -strip -colorspace CMYK -quality 90 "$img/jpeg-cmyk.jpg"
q -strip -colorspace Gray -depth 8 -define png:color-type=0 -define png:bit-depth=8 \
  -define png:exclude-chunks=date,time "$img/png-gray.png"
q -strip -colorspace Gray -alpha set -channel A -evaluate set 60% +channel -depth 8 \
  -define png:color-type=4 -define png:bit-depth=8 -define png:exclude-chunks=date,time "$img/png-gray-alpha.png"
q -strip -depth 16 -define png:exclude-chunks=date,time "PNG48:$img/png-16bit.png"
q -strip -fuzz 25% -transparent '#2040c0' -colors 16 -define png:exclude-chunks=date,time "PNG8:$img/png-palette-trns.png"
q -strip -depth 8 -interlace PNG -define png:color-type=2 -define png:exclude-chunks=date,time "$img/png-interlaced.png"
q -strip "$img/gif.gif"
cwebp -quiet -q 90 "$src" -o "$img/webp-lossy.webp"
cwebp -quiet -lossless "$src" -o "$img/webp-lossless.webp"
cat >"$img/svg-small.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="48" height="32" viewBox="0 0 48 32"><rect width="48" height="32" fill="#2a6"/><circle cx="24" cy="16" r="10" fill="#c22"/></svg>
SVG
cat >"$img/svg-fractional.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="48.3" height="31.7" viewBox="0 0 48 32"><rect width="48" height="32" fill="#a62"/></svg>
SVG
cat >"$img/svg-viewbox-only.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 20"><rect width="30" height="20" fill="#26a"/><circle cx="15" cy="10" r="6" fill="#fd2"/></svg>
SVG
# GIF whose first frame is smaller and offset on a larger logical screen.
magick -size 64x40 xc:'#ffffff' "$src" -resize 32x20 -geometry +16+10 -composite "$img/gif-base-tmp.png"
magick -dispose none -delay 10 -size 64x40 xc:none \( "$src" -resize 32x20 -repage +16+10 \) \
  -delete 0 -set page 64x40+16+10 "$img/gif-offset-frame.gif"
rm -f "$img/gif-base-tmp.png"
# Animated GIF: two frames.
magick -delay 20 "$src" \( "$src" -negate \) -loop 0 "$img/gif-animated.gif"
cat >"$img/svg-huge.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="9000" height="40" viewBox="0 0 9000 40"><rect width="9000" height="40" fill="#46a"/></svg>
SVG
for f in "$img"/*; do
  name=$(basename "$f")
  [ "$name" = src.png ] && continue
  case "$name" in
    *.jpg) mime=image/jpeg ;; *.png) mime=image/png ;; *.gif) mime=image/gif ;;
    *.webp) mime=image/webp ;; *.svg) mime=image/svg+xml ;;
  esac
  b64=$(base64 <"$f" | tr -d '\n')
  base=${name%.*}
  cat >"$out/pages/img-$base.html" <<HTML
<!doctype html><html><head></head>
<body><style>body{margin:8px;font:14px sans-serif}img{display:block;margin:4px}.w{width:120px}.bg{width:96px;height:60px;background:url(data:$mime;base64,$b64) 0 0/48px 30px repeat}</style><p>$name</p><img data-cid="natural" src="data:$mime;base64,$b64"><img data-cid="sized" class="w" src="data:$mime;base64,$b64"><div data-cid="bg" class="bg"></div></body></html>
HTML
done
(cd "$img" && shasum -a 256 * >"$out/IMAGES.sha256")
ls "$out/pages" | wc -l
