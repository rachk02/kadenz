/*!
 * Coque animée du portail des licences Kadenz : modules du gabarit d'origine (méga-menu,
 * menu mobile, indicateur des pilules, couleur de la barre, apparitions), pris tels quels
 * dans le paquet de la maquette ; GSAP, ScrollTrigger, SplitText et Lenis sont chargés
 * avant ce fichier, depuis leurs fichiers officiels (voir MENTIONS.md).
 */
(() => {
  if (!window.gsap) return;
  var oi = window.gsap, M5 = window.gsap, $4 = window.ScrollTrigger, P6 = window.SplitText, Te3 = window.Lenis;
  oi.registerPlugin($4, P6);
  // Méga-menu (NavMega)
  var t3 = document.querySelector(`[data-mega]`);
  var n3 = t3?.closest(`header`);
  var r3 = document.querySelector(`.navigation-content`);
  t3 && n3 && r3 && window.matchMedia(`(min-width: 992px) and (hover: hover)`).matches && i3(t3, n3, r3);
  function i3(t30, n34, r31) {
    let i31 = +!window.matchMedia(`(prefers-reduced-motion: reduce)`).matches, a29 = t30.querySelector(`[data-mega-panel]`), o22 = new Map(Array.from(t30.querySelectorAll(`[data-mega-section]`)).map((e35) => [e35.dataset.megaSection, e35])), s22 = [`entreprises`, `revendeurs`, `journal`], c17 = /* @__PURE__ */ new Map();
    if (r31.querySelectorAll(`.nav-menu a`).forEach((e35) => {
      let t31 = e35.dataset.megaCle;
      o22.has(t31) && c17.set(e35, t31);
    }), !c17.size) return;
    let l18 = () => {
      let e35 = n34.getBoundingClientRect(), t31 = r31.getBoundingClientRect();
      return { left: t31.left - e35.left, top: t31.bottom - e35.top + 8, width: t31.width };
    }, u17 = (e35) => {
      let t31 = o22.get(e35);
      return t31.style.width = `${l18().width}px`, t31.offsetHeight;
    }, d16 = null, f15 = 0, p15 = null, m14 = /* @__PURE__ */ new Map(), h12 = (e35, t31) => o22.get(e35).querySelectorAll(`[data-mega-strip] video`)[t31] ?? null, g14 = (e35) => {
      o22.forEach((t31, n35) => t31.querySelectorAll(`video`).forEach((t32, r32) => {
        n35 === e35 && r32 === (m14.get(n35) ?? 0) ? (t32.src || (t32.src = t32.dataset.src), t32.play().catch(() => {
        })) : t32.paused || t32.pause();
      }));
    }, _13 = (t31, n35, r32 = true) => {
      let a30 = o22.get(t31), s23 = a30.querySelector(`[data-mega-strip]`);
      a30.querySelectorAll(`[data-mega-index]`).forEach((e35, t32) => e35.classList.toggle(`is-hot`, t32 === n35));
      let c18 = s23.children[0].offsetHeight + parseFloat(getComputedStyle(s23).rowGap || `0`);
      oi.to(s23, { y: -n35 * c18, duration: r32 ? 0.7 * i31 : 0, ease: `power3.out`, overwrite: true }), m14.set(t31, n35), d16 === t31 && g14(t31), [n35 - 1, n35 + 1].forEach((e35) => {
        let n36 = h12(t31, e35);
        n36 && !n36.src && (n36.src = n36.dataset.src, n36.preload = `auto`);
      });
    };
    o22.forEach((e35, t31) => {
      e35.querySelectorAll(`[data-mega-index]`).forEach((e36, n35) => e36.addEventListener(`pointerenter`, () => _13(t31, n35))), _13(t31, 0, false);
    });
    let v13 = (t31, n35, r32 = 0) => {
      o22.forEach((a30, o23) => {
        o23 === t31 ? (a30.classList.add(`is-on`), oi.fromTo(a30, { opacity: 0, x: n35 * 16 }, { opacity: 1, x: 0, duration: 0.4 * i31, ease: `power3.out`, delay: (r32 + (n35 ? 0.14 : 0)) * i31, overwrite: true })) : a30.classList.contains(`is-on`) && oi.to(a30, { opacity: 0, x: -n35 * 12, duration: 0.16 * i31, ease: `power2.in`, overwrite: true, onComplete: () => a30.classList.remove(`is-on`) });
      });
    }, y13 = (t31, n35 = 0.45) => {
      f15 = u17(t31), oi.to(a29, { height: f15, duration: n35 * i31, ease: `power3.inOut`, overwrite: `auto` });
    }, b13 = (e35) => o22.forEach((t31, n35) => t31.querySelectorAll(`a`).forEach((t32) => t32.setAttribute(`tabindex`, n35 === e35 ? `0` : `-1`))), x13 = (n35) => {
      if (n35 === d16) return;
      let r32 = d16 === null, o23 = r32 ? 0 : Math.sign(s22.indexOf(n35) - s22.indexOf(d16));
      d16 = n35, t30.removeAttribute(`aria-hidden`), b13(n35), g14(n35), r32 ? (p15?.kill(), f15 = u17(n35), oi.set(a29, { ...l18(), height: f15, visibility: `visible`, clipPath: `inset(0% 0% 100% 0% round 20px)` }), p15 = oi.timeline().to(a29, { clipPath: `inset(0% 0% 0% 0% round 20px)`, duration: 0.55 * i31, ease: `power3.out` }), v13(n35, 0, 0.1)) : (y13(n35), v13(n35, o23));
    }, S12 = () => {
      d16 !== null && (d16 = null, t30.setAttribute(`aria-hidden`, `true`), b13(null), p15?.kill(), o22.forEach((t31) => {
        t31.classList.contains(`is-on`) && oi.to(t31, { opacity: 0, duration: 0.2 * i31, ease: `power2.in`, overwrite: true, onComplete: () => t31.classList.remove(`is-on`) });
      }), p15 = oi.timeline({ onComplete: () => {
        oi.set(a29, { visibility: `hidden` }), g14(null);
      } }).to(a29, { clipPath: `inset(0% 0% 100% 0% round 20px)`, duration: 0.4 * i31, ease: `power3.in` }, 0.05 * i31));
    }, C12 = new ResizeObserver(() => {
      d16 && u17(d16) !== f15 && y13(d16, 0.3);
    });
    o22.forEach((e35) => C12.observe(e35));
    let w10 = 0, T10 = 0, E9 = () => {
      clearTimeout(w10), clearTimeout(T10);
    }, D9 = (e35 = 220) => {
      E9(), T10 = window.setTimeout(S12, e35);
    };
    c17.forEach((e35, t31) => {
      t31.addEventListener(`pointerenter`, () => {
        E9(), w10 = window.setTimeout(() => x13(e35), d16 ? 30 : 80);
      }), t31.addEventListener(`pointerleave`, () => D9()), t31.addEventListener(`focus`, () => {
        E9(), x13(e35);
      }), t31.addEventListener(`click`, () => S12());
    }), r31.querySelectorAll(`[data-indicator-item], .nav-cta`).forEach((e35) => {
      c17.has(e35) || e35.addEventListener(`pointerenter`, () => {
        d16 ? D9(120) : clearTimeout(w10);
      });
    }), a29.addEventListener(`pointerenter`, E9), a29.addEventListener(`pointerleave`, () => D9()), a29.addEventListener(`click`, (e35) => {
      e35.target.closest(`a`) && S12();
    }), a29.addEventListener(`focusout`, (e35) => {
      !a29.contains(e35.relatedTarget) && ![...c17.keys()].includes(e35.relatedTarget) && D9();
    }), document.addEventListener(`keydown`, (e35) => {
      e35.key === `Escape` && S12();
    }), window.addEventListener(`resize`, S12), window.addEventListener(`pageshow`, (e35) => {
      e35.persisted && (d16 = `x`, S12());
    });
  }

  // Indicateur des pilules (pill-indicator)
  var t11 = window.matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  function n11(n34, r31 = `[data-indicator-item]`) {
    let i31 = n34.querySelector(`:scope > [data-indicator]`);
    i31 || (i31 = document.createElement(`span`), i31.className = `pill-indicator`, i31.setAttribute(`aria-hidden`, `true`), i31.dataset.indicator = ``, n34.prepend(i31));
    let a29 = i31, o22 = () => Array.from(n34.querySelectorAll(r31)), s22 = o22().find((e35) => e35.classList.contains(`is-active`)) ?? null, c17 = null, l18 = 0, u17 = (e35) => {
      let t30 = n34.getBoundingClientRect(), r32 = e35.getBoundingClientRect(), i32 = Math.min(parseFloat(getComputedStyle(e35).borderTopLeftRadius) || 0, r32.height / 2);
      return { x: r32.left - t30.left + n34.scrollLeft, y: r32.top - t30.top + n34.scrollTop, width: r32.width, height: r32.height, borderRadius: i32 };
    }, d16 = (e35) => {
      o22().forEach((t30) => t30.classList.toggle(`is-lit`, t30 === e35));
    };
    function f15(n35, r32 = true) {
      if (n35 === c17 && r32) return;
      let i32 = c17 !== null;
      if (c17 = n35, d16(n35), oi.killTweensOf(a29), !n35) {
        oi.to(a29, { opacity: 0, scale: 0.92, duration: r32 && !t11 ? 0.35 : 0, ease: `power2.out` });
        return;
      }
      let o23 = u17(n35);
      !i32 || !r32 || t11 ? (oi.set(a29, { ...o23 }), oi.fromTo(a29, { opacity: 0, scale: 0.94 }, { opacity: 1, scale: 1, duration: r32 && !t11 ? 0.45 : 0, ease: `power3.out` })) : oi.to(a29, { ...o23, opacity: 1, scale: 1, duration: 0.6, ease: `power3.out` });
    }
    n34.addEventListener(`pointerover`, (e35) => {
      if (e35.pointerType === `touch`) return;
      let t30 = e35.target.closest(r31);
      t30 && n34.contains(t30) ? (clearTimeout(l18), f15(t30)) : (clearTimeout(l18), l18 = window.setTimeout(() => f15(s22), 140));
    }), n34.addEventListener(`pointerleave`, () => {
      clearTimeout(l18), l18 = window.setTimeout(() => f15(s22), 80);
    }), n34.addEventListener(`focusin`, (e35) => {
      let t30 = e35.target.closest(r31);
      t30 && f15(t30);
    }), n34.addEventListener(`focusout`, () => f15(s22));
    let p15 = () => {
      c17 && oi.set(a29, u17(c17));
    };
    return window.addEventListener(`resize`, p15), n34.addEventListener(`scroll`, p15, { passive: true, capture: true }), document.fonts?.ready.then(p15), f15(s22, false), n34.classList.add(`has-indicator`), { setActive(e35, t30 = true) {
      o22().forEach((t31) => t31.classList.toggle(`is-active`, t31 === e35)), s22 = e35, clearTimeout(l18), e35 && c17 === e35 ? d16(e35) : f15(e35, t30);
    }, refresh: p15, rest() {
      clearTimeout(l18), f15(s22);
    } };
  }

  // Apparitions au défilement (reveal)
  var t16 = { rise: 48, duration: 1, ease: `expo.out`, stagger: 0.06, titleEase: `power3.out`, titleDuration: 1.2, titleStagger: 0.08 };
  var n18 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var r16 = document.documentElement;
  var i18 = window.__pageShown ?? Promise.resolve();
  var a16 = false;
  var o13 = (() => {
    try {
      let e35 = `rv-seen:` + location.pathname;
      if (sessionStorage.getItem(e35)) return false;
      sessionStorage.setItem(e35, `1`);
    } catch {
    }
    return true;
  })();
  i18.then(() => setTimeout(() => {
    a16 = true, f11();
  }, 200));
  var s14 = /* @__PURE__ */ new Map();
  var c11 = /* @__PURE__ */ new Map();
  var l13 = [];
  var u13 = 0;
  var d12 = new IntersectionObserver((e35) => {
    for (let t30 of e35) t30.isIntersecting && (d12.unobserve(t30.target), l13.push(t30.target));
    l13.length && !u13 && (u13 = requestAnimationFrame(f11));
  }, { rootMargin: `0px 0px -8% 0px` });
  function f11() {
    if (u13 = 0, !a16 || !l13.length) return;
    let n34 = l13.sort((e35, t30) => {
      let n35 = e35.getBoundingClientRect(), r32 = t30.getBoundingClientRect();
      return Math.abs(n35.top - r32.top) > 4 ? n35.top - r32.top : n35.left - r32.left;
    });
    l13 = [];
    let r31 = n34.filter((e35) => !s14.has(e35));
    r31.length && oi.to(r31, { opacity: 1, y: 0, duration: t16.duration, ease: t16.ease, stagger: t16.stagger, clearProps: `opacity,transform,transition` });
    let i31 = 0;
    n34.filter((e35) => s14.has(e35)).forEach((n35) => {
      let r32 = s14.get(n35);
      s14.delete(n35);
      let a29 = o13 ? parseFloat(n35.dataset.revealDelay ?? ``) : NaN;
      oi.delayedCall(Number.isFinite(a29) ? a29 : i31++ * t16.titleStagger, r32);
    });
    for (let t30 of r31) c11.get(t30)?.forEach((n35) => {
      if (n35 === t30 || oi.getProperty(n35, `opacity`) === 1) return;
      let r32 = n35.getBoundingClientRect();
      (r32.right < 0 || r32.left > innerWidth) && (d12.unobserve(n35), oi.set(n35, { clearProps: `opacity,transform,transition` }));
    }), c11.delete(t30);
  }
  function p13(e35) {
    l13.push(e35), u13 || (u13 = requestAnimationFrame(f11));
  }
  function m13(r31) {
    !n18 && r31.length && (oi.set(r31, { opacity: 0, y: t16.rise, transition: `none` }), r31.forEach((e35) => {
      c11.set(e35, r31), d12.observe(e35);
    }));
  }
  function g12(n34) {
    let r31 = n34.dataset.reveal, i31 = (n34.closest(`section`) ?? n34).getBoundingClientRect().top < innerHeight, a29 = (r32, i32) => {
      oi.set(n34, r32), s14.set(n34, () => oi.to(n34, { ...i32, duration: t16.duration, ease: t16.ease }));
    };
    if (r31 === `fade`) a29({ opacity: 0, transition: `none` }, { opacity: 1, clearProps: `opacity,transition` });
    else if (r31 === `slide`) a29({ yPercent: 15 }, { yPercent: 0 });
    else if (r31 === `edge`) {
      let e35 = n34.closest(`section`) ?? document.body;
      a29({ y: Math.max(0, e35.getBoundingClientRect().bottom - n34.getBoundingClientRect().top) }, { y: 0, clearProps: `transform` });
    } else r31 === `settle` ? (oi.set(n34, { scale: 1.08 }), s14.set(n34, () => oi.to(n34, { scale: 1, duration: o13 ? 2.4 : t16.duration, ease: o13 ? `power2.out` : t16.ease, clearProps: `scale` }))) : a29({ opacity: 0, y: t16.rise, transition: `none` }, { opacity: 1, y: 0, clearProps: `opacity,transform,transition` });
    (r31 === `edge` || r31 === `settle`) && i31 ? p13(n34) : d12.observe(n34);
  }
  var _11 = { yPercent: 150 };
  function v11(t30) {
    return getComputedStyle(t30).display === `block` ? (oi.set(t30, { display: `flex`, flexDirection: `column` }), () => oi.set(t30, { clearProps: `display,flexDirection` })) : () => {
    };
  }
  function y11(n34) {
    let r31 = Array.from(n34.children).filter((e35) => e35.tagName !== `BR` && getComputedStyle(e35).display !== `inline`);
    if (r31.length > 1 && r31.length === Array.from(n34.children).filter((e35) => e35.tagName !== `BR`).length) {
      let i31 = r31.filter((e35) => !e35.classList.contains(`visually-hidden`)), a29 = i31.map((e35) => {
        let t30 = document.createElement(`span`);
        return t30.className = `rvline-mask`, e35.before(t30), t30.append(e35), t30;
      }), o22 = v11(n34);
      oi.set(i31, _11), s14.set(n34, () => oi.to(i31, { yPercent: 0, duration: t16.titleDuration, ease: t16.titleEase, stagger: t16.titleStagger, onComplete: () => {
        a29.forEach((e35) => e35.replaceWith(...e35.childNodes)), oi.set(i31, { clearProps: `transform` }), o22();
      } }));
    } else oi.set(n34, { opacity: 0 }), s14.set(n34, () => {
      let r32 = b11(n34), i31 = v11(n34);
      return oi.set(n34, { opacity: 1 }), oi.fromTo(r32.lines, _11, { yPercent: 0, duration: t16.titleDuration, ease: t16.titleEase, stagger: t16.titleStagger, onComplete: () => {
        r32.revert(), i31();
      } });
    });
    d12.observe(n34);
  }
  function b11(e35) {
    let t30 = Array.from(e35.childNodes), n34 = getComputedStyle(e35), r31 = parseFloat(n34.lineHeight) || parseFloat(n34.fontSize) * 1.2, i31 = [], a29 = document.createRange(), o22 = (e36) => e36 ? (e36.top + e36.bottom) / 2 : NaN, s22 = (e36, t31) => {
      e36.childNodes.forEach((e37) => {
        if (e37.nodeType === Node.TEXT_NODE) {
          let n35 = e37.textContent ?? ``;
          for (let r32 of n35.matchAll(/\s+|\S+?(?:[-\u2010\u2013\u2014]+\s*|\s+|$)/g)) {
            let n36 = r32[0];
            n36.trim() ? (a29.setStart(e37, r32.index), a29.setEnd(e37, r32.index + n36.trimEnd().length), i31.push({ node: e37, chain: t31, mid: o22(a29.getClientRects()[0]), text: n36 })) : i31.push({ node: e37, chain: t31, mid: NaN, text: n36 });
          }
        } else e37 instanceof Element && e37.tagName !== `BR` && (e37.textContent?.trim() ? s22(e37, [...t31, e37]) : i31.push({ node: e37, chain: t31, mid: o22(e37.getBoundingClientRect()) }));
      });
    };
    s22(e35, []);
    let c17 = [], l18 = -1 / 0;
    for (let e36 of i31) !Number.isNaN(e36.mid) && e36.mid > l18 + r31 / 2 && (c17.push([]), l18 = e36.mid), (c17[c17.length - 1] ?? (c17[0] = [])).push(e36);
    let u17 = c17.map((e36) => {
      let t31 = document.createElement(`span`);
      t31.className = `rvline`;
      let n35 = /* @__PURE__ */ new Map(), r32 = (e37) => e37.reduce((e38, t32) => {
        let r33 = n35.get(t32);
        return r33 || (r33 = t32.cloneNode(false), e38.append(r33), n35.set(t32, r33)), r33;
      }, t31);
      e36.forEach((e37) => r32(e37.chain).append(e37.text === void 0 ? e37.node.cloneNode(true) : document.createTextNode(e37.text)));
      let i32 = document.createElement(`span`);
      return i32.className = `rvline-mask`, i32.append(t31), { mask: i32, line: t31 };
    });
    return e35.replaceChildren(...u17.map((e36) => e36.mask)), { lines: u17.map((e36) => e36.line), revert: () => e35.replaceChildren(...t30) };
  }
  var x11 = (e35) => s14.has(e35);
  function S10(r31, i31) {
    !n18 && i31.length && (oi.set(i31, _11), s14.set(r31, () => oi.to(i31, { yPercent: 0, duration: t16.titleDuration, ease: t16.titleEase, stagger: t16.titleStagger })), d12.observe(r31));
  }
  function C10() {
    n18 || document.querySelectorAll(`[data-reveal]`).forEach((e35) => {
      let t30 = e35.dataset.reveal;
      t30 === `cards` ? m13(Array.from(e35.children).map((t31) => e35.hasAttribute(`data-slider`) ? t31.firstElementChild ?? t31 : t31)) : t30 === `title` ? y11(e35) : g12(e35);
    }), r16.classList.add(`rv-ready`);
  }
  document.fonts?.status === `loaded` || n18 ? C10() : (window.__fontsReady ?? document.fonts.ready).then(C10);

  // Barre, menu mobile, mesures (main)
  var Q5 = window.matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var $5 = null;
  var Re3 = () => $5;
  Q5 || ($5 = new Te3({ autoRaf: true, smoothWheel: true, autoResize: true }), $5.on(`scroll`, $4.update), window.addEventListener(`load`, () => $5?.resize()));
  function ze3() {
    let e35 = document.querySelector(`[data-nav-bar]`), t30 = Array.from(document.querySelectorAll(`[data-nav="inverted"]`));
    if (!e35) return;
    let n34 = false, r31 = () => {
      if (!n34) return;
      let r32 = e35.getBoundingClientRect(), i32 = r32.top + r32.height / 2, a30 = t30.some((e36) => {
        let t31 = e36.getBoundingClientRect();
        return t31.top <= i32 && t31.bottom > i32;
      });
      e35.classList.toggle(`is-light`, a30);
    };
    window.addEventListener(`scroll`, r31, { passive: true }), window.addEventListener(`resize`, r31), $5?.on(`scroll`, r31);
    let i31 = document.documentElement, a29 = () => {
      if (i31.classList.contains(`nav-instant`)) return requestAnimationFrame(a29);
      requestAnimationFrame(() => requestAnimationFrame(() => {
        n34 = true, r31();
      }));
    };
    (window.__pageShown ?? Promise.resolve()).then(a29);
  }
  function Be3() {
    let e35 = document.querySelector(`[data-menu]`), n34 = e35?.querySelector(`[data-menu-panel]`), r31 = document.querySelector(`.nav-toggle`), i31 = document.querySelector(`[data-menu-label]`);
    if (!e35 || !n34 || !r31) return;
    let a29 = matchMedia(`(prefers-reduced-motion: reduce)`).matches, o22 = new Map(Array.from(e35.querySelectorAll(`[data-mm-step]`)).map((e36) => [e36.dataset.mmStep, e36])), s22 = `root`, c17 = 0, l18 = (e36, r32) => {
      let i32 = o22.get(s22), l19 = o22.get(e36);
      i32 && l19 && e36 !== s22 && (clearTimeout(c17), s22 = e36, n34.style.height = `${n34.offsetHeight}px`, oi.to(i32, { opacity: 0, x: -r32 * 16, duration: a29 ? 0 : 0.18, ease: `power2.in`, overwrite: true, onComplete: () => {
        let e37 = i32.contains(document.activeElement);
        i32.hidden = true, l19.hidden = false, e37 && l19.querySelector(`a[href], button`)?.focus({ preventScroll: true }), oi.set(l19, { opacity: 0, x: r32 * 20 }), n34.style.height = `${l19.offsetHeight}px`, oi.to(l19, { opacity: 1, x: 0, duration: a29 ? 0 : 0.4, ease: `power3.out`, delay: a29 ? 0 : 0.08 }), c17 = window.setTimeout(() => {
          n34.style.height = ``;
        }, 500);
      } }));
    }, u17 = () => {
      o22.forEach((e36, n35) => {
        e36.hidden = n35 !== `root`, oi.set(e36, { clearProps: `opacity,transform` });
      }), s22 = `root`, n34.style.height = ``;
    }, d16 = (t30) => {
      r31.setAttribute(`aria-expanded`, String(t30)), i31 && (i31.textContent = t30 ? `Fermer` : `Menu`), t30 ? (u17(), e35.hidden = false, requestAnimationFrame(() => requestAnimationFrame(() => e35.classList.add(`is-open`))), $5?.stop()) : (e35.classList.remove(`is-open`), $5?.start(), setTimeout(() => {
        e35.classList.contains(`is-open`) || (e35.hidden = true, u17());
      }, 550));
    };
    document.querySelectorAll(`[data-menu-toggle]`).forEach((e36) => e36.addEventListener(`click`, () => d16(r31.getAttribute(`aria-expanded`) !== `true`))), e35.querySelectorAll(`[data-mm-open]`).forEach((e36) => e36.addEventListener(`click`, () => l18(e36.dataset.mmOpen, 1))), e35.querySelectorAll(`[data-mm-back]`).forEach((e36) => e36.addEventListener(`click`, () => l18(`root`, -1))), e35.querySelectorAll(`[data-menu-link]`).forEach((e36) => e36.addEventListener(`click`, () => d16(false))), document.addEventListener(`keydown`, (e36) => {
      if (r31.getAttribute(`aria-expanded`) !== `true`) return;
      if (e36.key === `Escape`) {
        d16(false), r31.focus({ preventScroll: true });
        return;
      }
      if (e36.key !== `Tab`) return;
      let t30 = [r31, ...Array.from(n34.querySelectorAll(`a[href], button:not([disabled])`)).filter((e37) => e37.getClientRects().length && !e37.closest(`[hidden]`))], i32 = t30.indexOf(document.activeElement), a30 = i32 < 0 ? t30[0] : t30[(i32 + (e36.shiftKey ? -1 : 1) + t30.length) % t30.length];
      e36.preventDefault(), a30.focus({ preventScroll: true });
    });
  }
  function Ve3() {
    let e35 = document.querySelector(`.navigation-content`);
    if (!e35) return;
    let t30 = document.documentElement.style, n34 = () => {
      t30.setProperty(`--nav-w`, `${e35.offsetWidth}px`), t30.setProperty(`--nav-h`, `${e35.offsetHeight}px`);
    };
    n34(), new ResizeObserver(n34).observe(e35);
  }
  function He3() {
    let e35 = document.querySelectorAll(`.grid-split:not(.page-split)`);
    if (!e35.length) return;
    let t30 = (e36) => {
      let t31 = e36.firstElementChild, n35 = e36.offsetHeight - (t31?.offsetHeight ?? 0);
      e36.classList.toggle(`is-long`, n35 > window.innerHeight * 0.85);
    }, n34 = new ResizeObserver((e36) => e36.forEach((e37) => t30(e37.target)));
    e35.forEach((e36) => n34.observe(e36)), window.addEventListener(`resize`, () => e35.forEach(t30));
  }

  ze3(), Be3(), Ve3(), He3(), document.querySelectorAll(`[data-nav-indicator]`).forEach((e35) => n11(e35));
})();
