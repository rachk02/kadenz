"""Regenere les SVG de la marque Kadenz (symbole, icones, logos).

Usage : python generer.py <dossier_sortie> <Manrope[wght].ttf> <chemins.json>
Prerequis : fontTools (pip install fonttools). Manrope est sous licence SIL OFL 1.1
(https://github.com/googlefonts/manrope) ; le mot-symbole est vectorise depuis la graisse 800.
"""
import math, json, sys, os
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen
OUT=sys.argv[1]; FONT=sys.argv[2]
INDIGO="#2d3a8c"; NUIT="#1a2259"; BLANC="#ffffff"
# ---------- symbole ----------
s=1.25; th=math.atan(s); wh=9/math.sin(th)
X0,X1,TOP,BASE,ox=10,19,6,42,39
lx=ox+2.5
up=lambda x0:(lambda x:TOP+(x0-x)*s); lo=lambda x0:(lambda x:BASE-(x0-x)*s)
ui,uo=up(ox-wh),up(ox); lo_o,lo_i=lo(lx),lo(lx-wh)
def inter(f,g):
    a=f(1)-f(0); b=f(0); c=g(1)-g(0); d=g(0); x=(d-b)/(a-c); return (x,f(x))
A=inter(lo_o,uo); B=inter(lo_i,uo)
# arm points at given y on inner/outer lower lines
xi=lambda y: (lx-wh)-(BASE-y)/s; xo=lambda y: lx-(BASE-y)/s
def fmt(pts): return "M"+" L".join(f"{x:.2f} {y:.2f}" for x,y in pts)+" Z"
stem=[(X0,TOP),(X1,TOP),(X1,BASE),(X0,BASE)]
XS=X1-2
upper=[(XS,ui(XS)),(ox-wh,TOP),(ox,TOP),(XS,uo(XS))]
gap=3.0
ystart=max(A[1],B[1]); L=BASE-ystart; seg=(L-2*gap)/3
y1=ystart+seg; y2=y1+gap; y3=y2+seg; y4=y3+gap
# first segment: from upper arm down to y1 (polygon B,A,(xo(y1),y1),(xi(y1),y1)) - B lies at stem side
OV=2.5
A2=(xo(A[1]-OV),A[1]-OV); B2=(xi(B[1]-OV),B[1]-OV)
s1=[B2,A2,(xo(y1),y1),(xi(y1),y1)]
s2=[(xi(y2),y2),(xo(y2),y2),(xo(y3),y3),(xi(y3),y3)]
s3=[(xi(y4),y4),(xo(y4),y4),(lx,BASE),(lx-wh,BASE)]
solid=[B2,A2,(lx,BASE),(lx-wh,BASE)]
D_CAD=" ".join(fmt(p) for p in (stem,upper,s1,s2,s3))
D_SOL=" ".join(fmt(p) for p in (stem,upper,solid))
KX0,KY0,KX1,KY1=X0,TOP,lx,BASE   # bounds
# ---------- mot-symbole ----------
f=TTFont(FONT); f=instantiateVariableFont(f,{"wght":800})
gs=f.getGlyphSet(); cmap=f.getBestCmap(); upm=f["head"].unitsPerEm
cap=f["OS/2"].sCapHeight
kern={}
def word(text,track=0):
    pen=SVGPathPen(gs); x=0
    for ch in text:
        g=cmap[ord(ch)]
        tp=TransformPen(pen,(1,0,0,-1,x,0)); gs[g].draw(tp)
        x+=gs[g].width+track
    return pen.getCommands(), x-track
WD,WW=word("Kadenz",track=-10)
def wordmark(color,x,y,h):
    # place so that cap height == h, baseline at y
    k=h/cap
    return f'<path fill="{color}" transform="translate({x:.2f} {y:.2f}) scale({k:.5f})" d="{WD}"/>', WW*k
def tile(x,y,size,bg,fg,d=D_CAD,r=3):
    k=size/48
    return (f'<rect x="{x}" y="{y}" width="{size}" height="{size}" rx="{r*k:.2f}" fill="{bg}"/>'
            f'<path fill="{fg}" transform="translate({x+(-1.75)*k:.2f} {y:.2f}) scale({k:.4f})" d="{d}"/>')
def svg(w,h,body,title):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.2f} {h:.2f}" role="img" aria-label="{title}"><title>{title}</title>{body}</svg>\n'
files={}
# symbole seul (K, viewBox serré)
sym=lambda c,d=D_CAD: svg(KX1-KX0,KY1-KY0,f'<path fill="{c}" transform="translate({-KX0} {-KY0})" d="{d}"/>',"Kadenz")
files["kadenz-symbole.svg"]=sym(INDIGO)
files["kadenz-symbole-blanc.svg"]=sym(BLANC)
files["kadenz-symbole-noir.svg"]=sym("#000000")
files["kadenz-icone.svg"]=svg(48,48,tile(0,0,48,INDIGO,BLANC),"Kadenz")
files["kadenz-icone-petite.svg"]=svg(48,48,tile(0,0,48,INDIGO,BLANC,D_SOL),"Kadenz")
# logo horizontal : tuile 48 + mot (hauteur de capitale 22)
def horiz(bg,fg,wcol):
    t=tile(0,0,48,bg,fg); wm,ww=wordmark(wcol,62,35,22)
    return svg(62+ww,48,t+wm,"Kadenz")
files["kadenz-logo-horizontal.svg"]=horiz(INDIGO,BLANC,"#1f2933")
files["kadenz-logo-horizontal-negatif.svg"]=horiz(BLANC,INDIGO,BLANC)
files["kadenz-logo-horizontal-mono.svg"]=horiz("#000000",BLANC,"#000000")
def stack(bg,fg,wcol):
    wm,ww=wordmark(wcol,0,0,20); W=max(ww,64)
    t=tile((W-64)/2,0,64,bg,fg)
    wm,ww=wordmark(wcol,(W-ww)/2,64+18+20,20)
    return svg(W,64+18+20+4,t+wm,"Kadenz")
files["kadenz-logo-empile.svg"]=stack(INDIGO,BLANC,"#1f2933")
files["kadenz-logo-empile-negatif.svg"]=stack(BLANC,INDIGO,BLANC)
os.makedirs(OUT,exist_ok=True)
for n,c in files.items():
    open(os.path.join(OUT,n),"w",encoding="utf-8",newline="\n").write(c)
json.dump({"cad":D_CAD,"sol":D_SOL,"word":WD,"ww":WW,"cap":cap,"kb":[KX0,KY0,KX1,KY1]},open(sys.argv[3],"w"))
print("ok", WW, cap, upm)
