#!/usr/bin/env python3
"""Writes src/html_image_srcset_cases.json (miaskiewicz/turbo-dom#71): one page of <img>
elements covering srcset x / w descriptors and their parse rules, sizes with media
conditions, and <picture> sources selected by type and media. Every image URL is
/img/<W>x<H>/<name>.png so the control's server (scripts/image-srcset-chrome-control.mjs)
answers a W x H PNG. Regenerate, then recapture the control:
  python3 scripts/make-image-srcset-cases.py   (from crates/turbo-dom-browser-env)
"""
import html
import json
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "html_image_srcset_cases.json")
def I(name, w, h): return f"/img/{w}x{h}/{name}.png"
A=I('a',100,50); B=I('b',200,100); C=I('c',300,150); D=I('d',400,200); S=I('s',100,50)
W4=I('w400',400,200); W8=I('w800',800,400); W16=I('w1600',1600,800)
cases=[]
def img(id, **attrs):
    a=''.join(f' {k.replace("_","-")}="{html.escape(v, quote=True)}"' for k,v in attrs.items())
    return f'<img id="{id}"{a}>'
def add(id, markup): cases.append({"id":id,"html":markup})
def addimg(id, **attrs): add(id, img(id, **attrs))
# x descriptors
addimg('x1', srcset=f'{A} 1x, {B} 2x')
addimg('x-src', src=S, srcset=f'{B} 2x')
addimg('x-src-1x', src=S, srcset=f'{A} 1x, {B} 2x')
addimg('x-15', srcset=f'{A} 1x, {B} 1.5x, {C} 3x')
addimg('x-geo', srcset=f'{A} 1.5x, {B} 2.5x')
addimg('x-geo2', srcset=f'{A} 1.7x, {B} 2.5x')
addimg('x-low', srcset=f'{A} 0.5x, {B} 0.8x')
addimg('x-dup', srcset=f'{A} 1x, {B} 1x, {C} 2x')
addimg('x-default', srcset=f'{A}, {B} 2x')
addimg('x-odd', srcset=f'{I("odd",301,201)} 2x')
addimg('x-third', srcset=f'{C} 3x')
# srcset parsing
addimg('p-commas', srcset=f'{A},{B} 2x')
addimg('p-trailing', srcset=f'{A} 1x,, , {B} 2x,')
addimg('p-paren', srcset=f'{A} 1x (a, b), {B} 2x')
addimg('p-h-only', srcset=f'{A} 100h, {B} 2x')
addimg('p-wh', srcset=f'{W4} 400w 200h, {W8} 800w', sizes='300px')
addimg('p-wx', srcset=f'{A} 400w 2x, {B} 1x')
addimg('p-two-x', srcset=f'{A} 1x 2x, {B} 3x')
addimg('p-zero', srcset=f'{A} 0x, {B} 2x')
addimg('p-neg', srcset=f'{A} -1x, {B} 2x')
addimg('p-0w', srcset=f'{A} 0w, {W8} 800w')
addimg('p-float-w', srcset=f'{W4} 400.5w, {W8} 800w', sizes='300px')
addimg('p-exp', srcset=f'{A} 1e0x, {B} 2e0x')
addimg('p-junk', srcset=f'{A} 2q, {B} 2x')
addimg('p-empty', src=S, srcset='')
addimg('p-blank', src=S, srcset=' , ,')
addimg('p-data', srcset='data:image/png;base64,iVBORw0KGgo= 1x, '+B+' 2x')
# w descriptors + sizes
addimg('w-100vw', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w')
addimg('w-px', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='300px')
addimg('w-media', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(max-width: 600px) 100px, (min-width: 601px) 500px, 50px')
addimg('w-media-miss', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(max-width: 100px) 100px, 200px')
addimg('w-calc', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='calc(50vw - 12px)')
addimg('w-em', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='20em')
addimg('w-vh', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='50vh')
addimg('w-invalid', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(min-width: 1px) foo, 300px')
addimg('w-negative', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='-10px, 400px')
addimg('w-percent', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='50%')
addimg('w-zero', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='0')
addimg('w-auto', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='auto')
addimg('w-not', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(not (min-width: 2000px)) 300px, 10px')
addimg('w-or', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(min-width: 2000px) or (orientation: landscape) 200px, 10px')
addimg('w-and', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(min-width: 500px) and (max-height: 500px) 200px, 700px')
addimg('w-range', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(400px <= width <= 2000px) 250px, 10px')
addimg('w-range-gt', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(width > 1024px) 250px, 600px')
addimg('w-resolution', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(min-resolution: 2dppx) 100px, 400px')
addimg('w-webkit-dpr', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(-webkit-min-device-pixel-ratio: 1.5) 100px, 400px')
addimg('w-media-type', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='screen 100px, 300px')
addimg('w-unknown-feature', srcset=f'{W4} 400w, {W8} 800w, {W16} 1600w', sizes='(foo: bar) 100px, 300px')
addimg('w-src-ignored', src=S, srcset=f'{W4} 400w, {W8} 800w', sizes='300px')
addimg('w-mixed', srcset=f'{W4} 400w, {B} 2x', sizes='300px')
# pictures
def pic(id, sources, img_attrs, between=''):
    s=''.join('<source'+''.join(f' {k}="{html.escape(v, quote=True)}"' for k,v in src.items())+'>' for src in sources)
    add(id, f'<picture>{s}{between}{img(id, **img_attrs)}</picture>')
pic('pic-type', [{'type':'image/x-unknown','srcset':I('u',10,10)},{'type':'image/webp','srcset':I('w',20,10)}], {'src':S})
for n,t in [('avif','image/avif'),('jxl','image/jxl'),('svg','image/svg+xml'),('tiff','image/tiff'),('bmp','image/bmp'),('ico','image/x-icon'),('vnd-ico','image/vnd.microsoft.icon'),('gif','image/gif'),('apng','image/apng'),('upper','IMAGE/PNG'),('params','image/png;q=1'),('spaces',' image/png '),('empty',''),('bare','png'),('jpg','image/jpg'),('pjpeg','image/pjpeg'),('heic','image/heic'),('jpeg','image/jpeg'),('xpng','image/x-png'),('text','text/plain')]:
    pic('pic-t-'+n, [{'type':t,'srcset':I('t-'+n,30,10)}], {'src':S})
pic('pic-media', [{'media':'(min-width: 2000px)','srcset':I('m2000',10,10)},{'media':'(min-width: 500px)','srcset':I('m500',20,10)}], {'src':S})
for n,m in [('list','print, (min-width: 10px)'),('print','print'),('not-screen','not screen'),('portrait','screen and (orientation: portrait)'),('empty',''),('invalid','garbage!'),('only','only screen and (min-width: 1px)'),('all','all'),('dpr','(min-resolution: 2dppx)'),('max-height','(max-height: 700px)')]:
    pic('pic-m-'+n, [{'media':m,'srcset':I('m-'+n,40,10)}], {'src':S})
pic('pic-sizes', [{'srcset':f'{W4} 400w, {W8} 800w, {W16} 1600w','sizes':'300px'}], {'src':S, 'sizes':'1000px'})
pic('pic-img-sizes', [], {'srcset':f'{W4} 400w, {W8} 800w, {W16} 1600w','sizes':'300px'})
pic('pic-src-attr', [{'src':I('srcattr',10,10)}], {'src':S})
pic('pic-empty-srcset', [{'srcset':''},{'srcset':I('second',10,10)}], {'src':S})
pic('pic-dpr', [{'srcset':f'{A} 1x, {B} 2x'}], {'src':S})
pic('pic-blank-srcset', [{'srcset':' , '},{'srcset':I('blank2',10,10)}], {'src':S})
add('pic-after', f'<picture>{img("pic-after", src=S)}<source srcset="{I("after",10,10)}"></picture>')
add('pic-nested', f'<picture><source srcset="{I("nested",10,10)}"><div>{img("pic-nested", src=S)}</div></picture>')
pic('pic-comment', [{'srcset':I('comment',10,10)}], {'src':S}, between='<!-- c -->text')
addimg('img-src', src=S)
addimg('img-none')
addimg('img-empty-src', src='')
addimg('img-relative', srcset='rel.png 2x, ../up.png 1x')
page='<!DOCTYPE html><html><head><style>img{display:block}</style></head><body>'+''.join(c['html'] for c in cases)+'</body></html>'
out={"viewport":[1024,768],"html":page,"ids":[c['id'] for c in cases]}
json.dump(out, open(OUT,'w'), indent=1)
print(len(cases), 'cases ->', os.path.normpath(OUT))
