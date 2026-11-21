/*!
 * Coque animée du portail des licences Kadenz : modules du gabarit d'origine (méga-menu,
 * menu mobile, indicateur des pilules, couleur de la barre, apparitions, sections de la
 * page Entreprises), pris tels quels dans le paquet de la maquette ; GSAP et ses modules
 * (ScrollTrigger, SplitText, Observer, CustomEase) et Lenis sont chargés
 * avant ce fichier, depuis leurs fichiers officiels (voir MENTIONS-gabarit.md).
 */
(() => {
  if (!window.gsap) return;
  var oi = window.gsap, M5 = window.gsap, $4 = window.ScrollTrigger, P6 = window.SplitText, Te3 = window.Lenis,
    j5 = window.Observer, Y5 = window.CustomEase;
  oi.registerPlugin($4, P6, j5, Y5), Y5.create(`slideshow-wipe`, `0.6, 0.08, 0.02, 0.99`);
  var __defProp = Object.defineProperty;
  var __typeError = (msg) => {
    throw TypeError(msg);
  };
  var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
  var __accessCheck = (obj, member, msg) => member.has(obj) || __typeError("Cannot " + msg);
  var __privateGet = (obj, member, getter) => (__accessCheck(obj, member, "read from private field"), getter ? getter.call(obj) : member.get(obj));
  var __privateAdd = (obj, member, value) => member.has(obj) ? __typeError("Cannot add the same private member more than once") : member instanceof WeakSet ? member.add(obj) : member.set(obj, value);
  var __privateSet = (obj, member, value, setter) => (__accessCheck(obj, member, "write to private field"), setter ? setter.call(obj, value) : member.set(obj, value), value);
  var __privateMethod = (obj, member, method) => (__accessCheck(obj, member, "access private method"), method);
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

  // Barre, menu mobile, mesures, diaporama, phrase qui tourne (main)
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
  function Ue3(e35) {
    let r31 = Array.from(e35.querySelectorAll(`[data-slideshow="slide"]`)), i31 = Array.from(e35.querySelectorAll(`[data-slideshow="parallax"]`)), a29 = Array.from(e35.querySelectorAll(`[data-slideshow="thumb"]`)), o22 = document.querySelector(`[data-caption-line]`), s22 = r31.length;
    if (s22 < 2) {
      Q5 || e35.querySelectorAll(`video`).forEach((e36) => {
        getComputedStyle(e36).display !== `none` && (e36.preload = `auto`, e36.play().catch(() => {
        }));
      });
      return;
    }
    let c17 = 0, l18 = false, u17 = 0.9, d16 = parseFloat(e35.dataset.slideDuration || `5`) || 5, f15 = null, p15 = false, m14 = true;
    new IntersectionObserver(([e36]) => {
      m14 = e36.isIntersecting, f15?.paused(p15 || !m14);
    }).observe(e35);
    let h12 = o22?.parentElement ?? null, g14 = (e36) => h12?.classList.toggle(`has-link`, !!r31[e36].dataset.href);
    g14(0), h12?.addEventListener(`click`, () => {
      let e36 = r31[c17].dataset.href;
      e36 && (/^https?:/.test(e36) ? window.open(e36, `_blank`, `noopener`) : (window.__pageGo ?? ((e37) => location.href = e37))(e36));
    });
    let _13 = a29.map((e36) => {
      let t30 = document.createElement(`div`);
      return t30.className = `thumb-progress`, e36.appendChild(t30), t30;
    }), v13 = (e36) => r31[e36].dataset.caption || ``, y13 = r31.map((e36) => Array.from(e36.querySelectorAll(`video`))), b13 = (e36) => y13[e36].forEach((e37) => {
      getComputedStyle(e37).display === `none` || Q5 && !p15 || (e37.preload = `auto`, e37.play().catch(() => {
      }));
    }), x13 = (e36) => y13[e36].forEach((e37) => e37.pause());
    window.__pageShown?.then(() => b13(0)) ?? b13(0);
    function S12(e36) {
      if (!o22) return;
      let n34 = o22.parentNode, r32 = o22.cloneNode(false);
      r32.textContent = v13(e36), oi.set(r32, { position: `absolute`, top: `0.15em`, left: 0, yPercent: 130 }), n34.appendChild(r32);
      let i32 = o22;
      oi.to(i32, { yPercent: -130, duration: 0.6, ease: `slideshow-wipe` }), oi.to(r32, { yPercent: 0, duration: 0.6, ease: `slideshow-wipe`, onComplete: () => {
        oi.set(r32, { position: ``, top: ``, left: `` }), i32.remove();
      } }), o22 = r32;
    }
    function C12() {
      f15?.kill(), _13.forEach((e36) => oi.set(e36, { scaleX: 0 }));
    }
    let w10 = (e36) => {
      e36 !== p15 && (p15 = e36, f15?.paused(p15 || !m14), Q5 && (e36 ? b13 : x13)(c17));
    };
    e35.addEventListener(`pointerenter`, (e36) => {
      e36.pointerType === `mouse` && w10(true);
    }), e35.addEventListener(`pointerleave`, (t30) => {
      t30.pointerType === `mouse` && !e35.contains(document.activeElement) && w10(false);
    }), e35.addEventListener(`focusin`, () => w10(true)), e35.addEventListener(`focusout`, (t30) => {
      e35.contains(t30.relatedTarget) || w10(e35.matches(`:hover`));
    });
    function T10() {
      if (C12(), Q5) return;
      let e36 = _13[c17];
      e36 && (oi.killTweensOf(e36), f15 = oi.fromTo(e36, { scaleX: 0 }, { scaleX: 1, duration: d16, ease: `none`, onComplete: () => {
        l18 || E9(1);
      } }), (p15 || !m14) && f15.pause());
    }
    function E9(e36, n34 = null) {
      if (l18) return;
      l18 = true, O8.disable(), C12();
      let o23 = c17;
      c17 = n34 ?? (e36 === 1 ? (c17 + 1) % s22 : (c17 - 1 + s22) % s22), oi.timeline({ defaults: { duration: u17, ease: `slideshow-wipe` }, onStart: () => {
        r31[c17].classList.add(`is--current`), a29[o23]?.classList.remove(`is--current`), a29[c17]?.classList.add(`is--current`), S12(c17), b13(c17), g14(c17);
      }, onComplete: () => {
        r31[o23].classList.remove(`is--current`), x13(o23), l18 = false, setTimeout(() => O8.enable(), u17 * 1e3), T10();
      } }).to(r31[o23], { xPercent: -e36 * 100 }, 0).to(i31[o23], { xPercent: e36 * 50 }, 0).fromTo(r31[c17], { xPercent: e36 * 100 }, { xPercent: 0 }, 0).fromTo(i31[c17], { xPercent: -e36 * 50 }, { xPercent: 0 }, 0);
    }
    a29.forEach((e36, t30) => e36.addEventListener(`click`, () => {
      t30 === c17 || l18 || E9(t30 > c17 ? 1 : -1, t30);
    }));
    let D9 = window.matchMedia(`(min-width: 992px)`).matches, O8 = j5.create({ target: e35, type: D9 ? `touch,pointer` : `wheel,touch,pointer`, onLeft: () => {
      l18 || E9(1);
    }, onRight: () => {
      l18 || E9(-1);
    }, tolerance: 10 });
    T10();
  }
  function We3() {
    document.querySelectorAll(`[data-rotating-title]`).forEach((e35) => {
      let t30 = parseFloat(e35.dataset.stepDuration || `1.75`), n34 = e35.innerHTML, r31 = null, i31 = () => {
      }, a29 = 0, o22 = () => {
        i31(), r31?.revert(), e35.innerHTML = n34, e35.querySelectorAll(`.intro-br, .intro-br-m, .intro-br-all`).forEach((e36) => {
          getComputedStyle(e36).display === `none` && e36.remove();
        }), a29 = e35.offsetWidth, r31 = P6.create(e35, { type: `lines`, mask: `lines`, linesClass: `rotating-line` }), (!e35.dataset.revealed || x11(e35)) && (e35.dataset.revealed = ``, S10(e35, r31.lines));
        let o23 = [...e35.querySelectorAll(`[data-rotating-words]`)], s23 = o23.find((e36) => e36.textContent?.trim()) ?? o23[0];
        o23.forEach((e36) => {
          e36 !== s23 && e36.remove();
        });
        let c17 = (s23?.dataset.rotatingWords || ``).split(`,`).map((e36) => e36.trim()).filter(Boolean);
        i31 = s23 && c17.length ? i19(s23, { words: c17, interval: t30, fit: true }) : () => {
        };
      };
      o22(), document.fonts?.ready.then(o22);
      let s22 = 0;
      new ResizeObserver(() => {
        clearTimeout(s22), s22 = window.setTimeout(() => {
          e35.offsetWidth !== a29 && o22();
        }, 150);
      }).observe(e35);
    });
  }

  ze3(), Be3(), Ve3(), He3(), document.querySelectorAll(`[data-nav-indicator]`).forEach((e35) => n11(e35));
  // odometer.B1bsPzO3.js
  var n19 = 0.7;
  var r17 = `power3.inOut`;
  function i19(t30, i31 = {}) {
    t30.__odo?.();
    let a29 = i31.interval ?? (parseFloat(t30.dataset.interval || ``) || 1.5), o22 = i31.fit ?? t30.hasAttribute(`data-fit`), s22 = i31.manual ?? t30.hasAttribute(`data-manual`);
    if (i31.words?.length) {
      t30.textContent = ``;
      for (let e36 of i31.words) {
        let n34 = document.createElement(`span`);
        n34.className = `odo-roll__w`, n34.setAttribute(`aria-hidden`, `true`), n34.textContent = e36, t30.append(n34);
      }
      let e35 = document.createElement(`span`);
      e35.className = `visually-hidden`, e35.textContent = i31.words.join(`, `), t30.append(e35);
    }
    t30.classList.add(`odo-roll`);
    let c17 = Array.from(t30.querySelectorAll(`.odo-roll__w`)), l18 = (e35) => e35.getBoundingClientRect().width, u17 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
    t30.classList.add(`is-ready`);
    let d16 = 0;
    if (oi.set(c17, { yPercent: 130 }), oi.set(c17[0], { yPercent: 0 }), o22 && (t30.style.width = l18(c17[0]) + `px`), s22) return t30.__odoTo = (i32, a30 = 1) => {
      if (i32 = (i32 % c17.length + c17.length) % c17.length, i32 === d16) return;
      let s23 = c17[d16], f16 = c17[i32];
      d16 = i32, u17 ? (oi.set(c17, { yPercent: 130 }), oi.set(f16, { yPercent: 0 }), o22 && (t30.style.width = l18(f16) + `px`)) : (oi.fromTo(f16, { yPercent: 130 * a30 }, { yPercent: 0, duration: n19, ease: r17, overwrite: true }), oi.to(s23, { yPercent: -130 * a30, duration: n19, ease: r17, overwrite: true }), o22 && oi.to(t30, { width: l18(f16), duration: n19, ease: r17, overwrite: true }));
    }, () => {
    };
    if (u17 || c17.length < 2) return () => {
    };
    let f15 = 0, p15 = true, m14 = true, h12 = () => {
      let i32 = c17[d16];
      d16 = (d16 + 1) % c17.length;
      let a30 = c17[d16];
      oi.fromTo(a30, { yPercent: 130 }, { yPercent: 0, duration: n19, ease: r17, overwrite: true }), oi.to(i32, { yPercent: -130, duration: n19, ease: r17, overwrite: true }), o22 && oi.to(t30, { width: l18(a30), duration: n19, ease: r17, overwrite: true }), g14();
    }, g14 = () => {
      clearTimeout(f15), m14 && p15 && !document.hidden && (f15 = window.setTimeout(h12, a29 * 1e3));
    }, _13 = new IntersectionObserver(([e35]) => {
      p15 = e35.isIntersecting, p15 ? g14() : clearTimeout(f15);
    });
    _13.observe(t30);
    let v13 = () => document.hidden ? clearTimeout(f15) : g14();
    document.addEventListener(`visibilitychange`, v13), document.fonts?.ready.then(() => {
      o22 && m14 && (t30.style.width = l18(c17[d16]) + `px`);
    }), g14();
    let y13 = () => {
      m14 = false, clearTimeout(f15), _13.disconnect(), document.removeEventListener(`visibilitychange`, v13), oi.killTweensOf([...c17, t30]);
    };
    return t30.__odo = y13, y13;
  }
  function a17(e35 = document) {
    e35.querySelectorAll(`[data-odometer]`).forEach((e36) => {
      e36.classList.contains(`is-ready`) || i19(e36);
    });
  }
  a17();

  // img-fade.DXLtvv7E.js
  document.querySelectorAll(`video[data-poster]`).forEach((e35) => {
    e35.poster = e35.dataset.poster;
  }), document.querySelectorAll(`img[data-fade][data-src]`).forEach((e35) => {
    e35.src = e35.dataset.src;
  });
  var e31 = /* @__PURE__ */ new WeakSet();
  var t17 = (t30) => {
    if (t30.classList.contains(`is-loaded`) || e31.has(t30)) return;
    e31.add(t30);
    let n34 = () => t30.classList.add(`is-loaded`);
    t30.decode ? t30.decode().then(n34, n34) : n34();
  };
  var n20 = () => document.querySelectorAll(`img[data-fade]:not(.is-loaded)`).forEach((e35) => {
    e35.complete && e35.naturalWidth && t17(e35);
  });
  document.addEventListener(`load`, (e35) => {
    let n34 = e35.target;
    n34 instanceof HTMLImageElement && n34.hasAttribute(`data-fade`) && t17(n34);
  }, true), n20(), requestAnimationFrame(n20), document.querySelectorAll(`video[data-fade]`).forEach((e35) => {
    if (e35.classList.contains(`is-loaded`)) return;
    let t30 = () => e35.classList.add(`is-loaded`);
    if (e35.readyState >= 2) return t30();
    if (e35.addEventListener(`loadeddata`, t30, { once: true }), e35.poster) {
      let n34 = new Image();
      n34.src = e35.poster, n34.decode().then(t30, () => {
      });
    }
  });

  // Hero.astro_astro_type_script_index_0_lang.CTRBnflW.js
  var t18 = window.matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  function n21() {
    let n34 = document.querySelector(`[data-news]`);
    if (!n34) return;
    let r31 = Array.from(n34.querySelectorAll(`[data-news-item]`)), i31 = r31.length, a29 = () => r31[0].offsetHeight + parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.5, o22 = (e35) => ({ y: e35 * 8, scale: 1 - e35 * 0.04, "--fill": [1, 0.55, 0.3][e35] ?? 0.3, "--ink": +!e35 }), s22 = (e35) => ({ y: -(i31 - 1 - e35) * a29(), scale: 1, "--fill": 1, "--ink": 1 });
    r31.forEach((t30, n35) => oi.set(t30, o22(n35)));
    let c17 = n34.querySelector(`[data-news-all]`);
    c17 && (oi.set(c17, { y: 8, opacity: 0 }), c17.addEventListener(`pointerenter`, () => h12()), c17.addEventListener(`pointerleave`, () => g14()));
    let l18 = Array.from(n34.querySelectorAll(`video[data-src]`)), u17 = (e35) => {
      e35.src || (e35.src = e35.dataset.src, e35.addEventListener(`loadeddata`, () => e35.classList.add(`is-ready`), { once: true })), t18 || e35.play().catch(() => {
      });
    }, d16 = r31[0].querySelector(`video[data-src]`);
    d16 && u17(d16);
    let f15 = false, p15 = (p16) => {
      p16 === f15 || i31 < 2 || (f15 = p16, n34.classList.toggle(`is-open`, p16), c17 && (c17.setAttribute(`tabindex`, p16 ? `0` : `-1`), oi.to(c17, { y: p16 ? -(i31 - 1) * a29() : 8, opacity: +!!p16, duration: t18 ? 0 : 0.6, ease: `power3.inOut`, delay: p16 && !t18 ? 0.08 : 0, overwrite: true })), r31.forEach((e35, t30) => e35.querySelector(`a`)?.setAttribute(`tabindex`, p16 || t30 === 0 ? `0` : `-1`)), p16 && l18.forEach(u17), r31.forEach((n35, r32) => oi.to(n35, { ...p16 ? s22(r32) : o22(r32), duration: t18 ? 0 : 0.6, ease: `power3.inOut`, overwrite: true })), p16 || oi.delayedCall(0.6, () => {
        f15 || l18.forEach((e35) => {
          e35 !== d16 && e35.pause();
        });
      }));
    }, m14 = 0, h12 = () => {
      clearTimeout(m14), p15(true);
    }, g14 = () => {
      clearTimeout(m14), m14 = window.setTimeout(() => p15(false), 200);
    };
    for (let e35 of r31) e35.addEventListener(`pointerenter`, h12), e35.addEventListener(`pointerleave`, g14);
    n34.addEventListener(`focusin`, h12), n34.addEventListener(`focusout`, (e35) => {
      n34.contains(e35.relatedTarget) || g14();
    });
  }
  function r18() {
    let e35 = document.querySelector(`[data-news-marquee]`);
    if (!e35) return;
    let t30 = () => {
      let t31 = e35.scrollWidth / 2;
      t31 && e35.style.setProperty(`--marquee-s`, `${Math.max(12, t31 / 45)}s`);
    };
    t30(), document.fonts?.ready.then(t30);
  }
  n21(), r18();

  // Slider.astro_astro_type_script_index_0_lang.7UMA2Izn.js
  var n22 = Object.create;
  var { getPrototypeOf: r19, defineProperty: i20, getOwnPropertyNames: a18 } = Object;
  var o14 = Object.prototype.hasOwnProperty;
  var s15 = ((e35, t30, s22) => {
    s22 = e35 == null ? {} : n22(r19(e35));
    let c17 = i20(s22, `default`, { value: e35, enumerable: true });
    for (let t31 of a18(e35)) o14.call(c17, t31) || i20(c17, t31, { get: () => e35[t31], enumerable: true });
    return c17;
  })((/* @__PURE__ */ ((e35, t30) => () => (t30 || e35((t30 = { exports: {} }).exports, t30), t30.exports))((e35, t30) => {
    (function(n34, r31) {
      typeof e35 == `object` && t30 !== void 0 ? t30.exports = r31() : typeof define == `function` && define.amd ? define(r31) : (n34 || self).virtualScroll = r31();
    })(e35, function() {
      var e36 = 0;
      function t31(t32) {
        return `__private_` + e36++ + `_` + t32;
      }
      function n34(e37, t32) {
        if (!Object.prototype.hasOwnProperty.call(e37, t32)) throw TypeError(`attempted to use private field on non-instance`);
        return e37;
      }
      function r31() {
      }
      r31.prototype = { on: function(e37, t32, n35) {
        var r32 = this.e || (this.e = {});
        return (r32[e37] || (r32[e37] = [])).push({ fn: t32, ctx: n35 }), this;
      }, once: function(e37, t32, n35) {
        var r32 = this;
        function i32() {
          r32.off(e37, i32), t32.apply(n35, arguments);
        }
        return i32._ = t32, this.on(e37, i32, n35);
      }, emit: function(e37) {
        for (var t32 = [].slice.call(arguments, 1), n35 = ((this.e || (this.e = {}))[e37] || []).slice(), r32 = 0, i32 = n35.length; r32 < i32; r32++) n35[r32].fn.apply(n35[r32].ctx, t32);
        return this;
      }, off: function(e37, t32) {
        var n35 = this.e || (this.e = {}), r32 = n35[e37], i32 = [];
        if (r32 && t32) for (var a30 = 0, o23 = r32.length; a30 < o23; a30++) r32[a30].fn !== t32 && r32[a30].fn._ !== t32 && i32.push(r32[a30]);
        return i32.length ? n35[e37] = i32 : delete n35[e37], this;
      } };
      var i31 = r31;
      i31.TinyEmitter = r31;
      var a29, o22 = `virtualscroll`, s22 = t31(`options`), c17 = t31(`el`), l18 = t31(`emitter`), u17 = t31(`event`), d16 = t31(`touchStart`), f15 = t31(`bodyTouchAction`);
      return function() {
        function e37(e38) {
          var t33 = this;
          Object.defineProperty(this, s22, { writable: true, value: void 0 }), Object.defineProperty(this, c17, { writable: true, value: void 0 }), Object.defineProperty(this, l18, { writable: true, value: void 0 }), Object.defineProperty(this, u17, { writable: true, value: void 0 }), Object.defineProperty(this, d16, { writable: true, value: void 0 }), Object.defineProperty(this, f15, { writable: true, value: void 0 }), this._onWheel = function(e39) {
            var r32 = n34(t33, s22)[s22], i32 = n34(t33, u17)[u17];
            i32.deltaX = e39.wheelDeltaX || -1 * e39.deltaX, i32.deltaY = e39.wheelDeltaY || -1 * e39.deltaY, a29.isFirefox && e39.deltaMode === 1 && (i32.deltaX *= r32.firefoxMultiplier, i32.deltaY *= r32.firefoxMultiplier), i32.deltaX *= r32.mouseMultiplier, i32.deltaY *= r32.mouseMultiplier, t33._notify(e39);
          }, this._onMouseWheel = function(e39) {
            var r32 = n34(t33, u17)[u17];
            r32.deltaX = e39.wheelDeltaX ? e39.wheelDeltaX : 0, r32.deltaY = e39.wheelDeltaY ? e39.wheelDeltaY : e39.wheelDelta, t33._notify(e39);
          }, this._onTouchStart = function(e39) {
            var r32 = e39.targetTouches ? e39.targetTouches[0] : e39;
            n34(t33, d16)[d16].x = r32.pageX, n34(t33, d16)[d16].y = r32.pageY;
          }, this._onTouchMove = function(e39) {
            var r32 = n34(t33, s22)[s22];
            r32.preventTouch && !e39.target.classList.contains(r32.unpreventTouchClass) && e39.preventDefault();
            var i32 = n34(t33, u17)[u17], a30 = e39.targetTouches ? e39.targetTouches[0] : e39;
            i32.deltaX = (a30.pageX - n34(t33, d16)[d16].x) * r32.touchMultiplier, i32.deltaY = (a30.pageY - n34(t33, d16)[d16].y) * r32.touchMultiplier, n34(t33, d16)[d16].x = a30.pageX, n34(t33, d16)[d16].y = a30.pageY, t33._notify(e39);
          }, this._onKeyDown = function(e39) {
            var r32 = n34(t33, u17)[u17];
            r32.deltaX = r32.deltaY = 0;
            var i32 = window.innerHeight - 40;
            switch (e39.keyCode) {
              case 37:
              case 38:
                r32.deltaY = n34(t33, s22)[s22].keyStep;
                break;
              case 39:
              case 40:
                r32.deltaY = -n34(t33, s22)[s22].keyStep;
                break;
              case 32:
                r32.deltaY = i32 * (e39.shiftKey ? 1 : -1);
                break;
              default:
                return;
            }
            t33._notify(e39);
          }, n34(this, c17)[c17] = window, e38 && e38.el && (n34(this, c17)[c17] = e38.el, delete e38.el), a29 || (a29 = { hasWheelEvent: `onwheel` in document, hasMouseWheelEvent: `onmousewheel` in document, hasTouch: `ontouchstart` in document, hasTouchWin: navigator.msMaxTouchPoints && navigator.msMaxTouchPoints > 1, hasPointer: !!window.navigator.msPointerEnabled, hasKeyDown: `onkeydown` in document, isFirefox: navigator.userAgent.indexOf(`Firefox`) > -1 }), n34(this, s22)[s22] = Object.assign({ mouseMultiplier: 1, touchMultiplier: 2, firefoxMultiplier: 15, keyStep: 120, preventTouch: false, unpreventTouchClass: `vs-touchmove-allowed`, useKeyboard: true, useTouch: true }, e38), n34(this, l18)[l18] = new i31(), n34(this, u17)[u17] = { y: 0, x: 0, deltaX: 0, deltaY: 0 }, n34(this, d16)[d16] = { x: null, y: null }, n34(this, f15)[f15] = null, n34(this, s22)[s22].passive !== void 0 && (this.listenerOptions = { passive: n34(this, s22)[s22].passive });
        }
        var t32 = e37.prototype;
        return t32._notify = function(e38) {
          var t33 = n34(this, u17)[u17];
          t33.x += t33.deltaX, t33.y += t33.deltaY, n34(this, l18)[l18].emit(o22, { x: t33.x, y: t33.y, deltaX: t33.deltaX, deltaY: t33.deltaY, originalEvent: e38 });
        }, t32._bind = function() {
          a29.hasWheelEvent && n34(this, c17)[c17].addEventListener(`wheel`, this._onWheel, this.listenerOptions), a29.hasMouseWheelEvent && n34(this, c17)[c17].addEventListener(`mousewheel`, this._onMouseWheel, this.listenerOptions), a29.hasTouch && n34(this, s22)[s22].useTouch && (n34(this, c17)[c17].addEventListener(`touchstart`, this._onTouchStart, this.listenerOptions), n34(this, c17)[c17].addEventListener(`touchmove`, this._onTouchMove, this.listenerOptions)), a29.hasPointer && a29.hasTouchWin && (n34(this, f15)[f15] = document.body.style.msTouchAction, document.body.style.msTouchAction = `none`, n34(this, c17)[c17].addEventListener(`MSPointerDown`, this._onTouchStart, true), n34(this, c17)[c17].addEventListener(`MSPointerMove`, this._onTouchMove, true)), a29.hasKeyDown && n34(this, s22)[s22].useKeyboard && document.addEventListener(`keydown`, this._onKeyDown);
        }, t32._unbind = function() {
          a29.hasWheelEvent && n34(this, c17)[c17].removeEventListener(`wheel`, this._onWheel), a29.hasMouseWheelEvent && n34(this, c17)[c17].removeEventListener(`mousewheel`, this._onMouseWheel), a29.hasTouch && (n34(this, c17)[c17].removeEventListener(`touchstart`, this._onTouchStart), n34(this, c17)[c17].removeEventListener(`touchmove`, this._onTouchMove)), a29.hasPointer && a29.hasTouchWin && (document.body.style.msTouchAction = n34(this, f15)[f15], n34(this, c17)[c17].removeEventListener(`MSPointerDown`, this._onTouchStart, true), n34(this, c17)[c17].removeEventListener(`MSPointerMove`, this._onTouchMove, true)), a29.hasKeyDown && n34(this, s22)[s22].useKeyboard && document.removeEventListener(`keydown`, this._onKeyDown);
        }, t32.on = function(e38, t33) {
          n34(this, l18)[l18].on(o22, e38, t33);
          var r32 = n34(this, l18)[l18].e;
          r32 && r32[o22] && r32[o22].length === 1 && this._bind();
        }, t32.off = function(e38, t33) {
          n34(this, l18)[l18].off(o22, e38, t33);
          var r32 = n34(this, l18)[l18].e;
          (!r32[o22] || r32[o22].length <= 0) && this._unbind();
        }, t32.destroy = function() {
          n34(this, l18)[l18].off(), this._unbind();
        }, e37;
      }();
    });
  }))(), 1);
  function c12(e35, t30, n34, r31) {
    let i31 = 1 - Math.exp(-n34 * r31);
    return e35 + (t30 - e35) * i31;
  }
  function l14(e35, t30) {
    let n34 = e35 % t30;
    return Math.abs(n34) > t30 / 2 && (n34 = n34 > 0 ? n34 - t30 : n34 + t30), n34;
  }
  var u14 = { infinite: true, snap: true, variableWidth: false, vertical: false, dragSensitivity: 5e-3, lerpFactor: 0.3, scrollSensitivity: 1, snapStrength: 0.1, speedDecay: 0.85, bounceLimit: 1, virtualScroll: { mouseMultiplier: 0.5, touchMultiplier: 2, firefoxMultiplier: 30, useKeyboard: false, passive: true }, setOffset: ({ itemWidth: e35, wrapperWidth: t30, itemHeight: n34, wrapperHeight: r31, vertical: i31 }) => i31 ? n34 : e35, scrollInput: false };
  var _e5, _t3, _n3, _r2, _i, _a, _o, _d_instances, s_fn, c_fn, l_fn, u_fn, d_fn, f_fn, p_fn, m_fn, h_fn, g_fn, __fn, v_fn, y_fn, b_fn, x_fn, S_fn, C_fn, w_fn, T_fn, _a2;
  var d13 = (_a2 = class {
    constructor(e35, t30 = {}) {
      __privateAdd(this, _d_instances);
      __publicField(this, "speed", 0);
      __privateAdd(this, _e5, 0);
      __privateAdd(this, _t3, 0);
      __privateAdd(this, _n3, 0);
      __publicField(this, "deltaTime", 0);
      __privateAdd(this, _r2, true);
      __privateAdd(this, _i, false);
      __privateAdd(this, _a, 0);
      __privateAdd(this, _o, 0);
      __publicField(this, "config");
      __publicField(this, "wrapper");
      __publicField(this, "items");
      __publicField(this, "viewport");
      __publicField(this, "itemWidths", []);
      __publicField(this, "itemOffsets", []);
      __publicField(this, "itemHeights", []);
      __publicField(this, "itemHeightOffsets", []);
      __publicField(this, "isDragging", false);
      __publicField(this, "isTouching", false);
      __publicField(this, "dragStart", 0);
      __publicField(this, "dragStartTarget", 0);
      __publicField(this, "isVisible", false);
      __publicField(this, "current", 0);
      __publicField(this, "target", 0);
      __publicField(this, "maxScroll", 0);
      __publicField(this, "resizeTimeout");
      __publicField(this, "virtualScroll");
      __publicField(this, "observer");
      __publicField(this, "touchStartY");
      __publicField(this, "touchStartX");
      __publicField(this, "touchPreviousX");
      __publicField(this, "touchPreviousY");
      __publicField(this, "scrollDirection");
      __publicField(this, "parallaxValues");
      __publicField(this, "webglValue", 0);
      __publicField(this, "onSlideChange");
      __publicField(this, "onResize");
      __publicField(this, "onUpdate");
      if (this.config = { ...u14, ...t30 }, t30.onSlideChange && (this.onSlideChange = t30.onSlideChange), t30.onResize && (this.onResize = t30.onResize), t30.onUpdate && (this.onUpdate = t30.onUpdate), delete this.config.onSlideChange, delete this.config.onResize, delete this.config.onUpdate, this.wrapper = e35, this.items = [...e35.children], this.current = 0, this.target = 0, this.isDragging = false, this.isTouching = false, this.dragStart = 0, this.dragStartTarget = 0, this.isVisible = false, __privateSet(this, _a, 0), __privateSet(this, _o, 0), __privateMethod(this, _d_instances, c_fn).call(this), __privateMethod(this, _d_instances, s_fn).call(this), __privateMethod(this, _d_instances, l_fn).call(this), this.wrapper.style.cursor = `grab`, __privateMethod(this, _d_instances, c_fn).call(this), __privateMethod(this, _d_instances, d_fn).call(this), this.config.variableWidth && !this.config.infinite && this.items.length > 0) {
        let e36 = __privateMethod(this, _d_instances, v_fn).call(this, 0);
        this.target = e36, this.current = e36, __privateMethod(this, _d_instances, S_fn).call(this);
      }
    }
    update() {
      if (!this.isVisible || !__privateGet(this, _r2)) return;
      let e35 = performance.now();
      if (this.deltaTime = (e35 - __privateGet(this, _n3)) / 1e3, __privateSet(this, _n3, e35), this.config.snap && !this.isDragging) {
        if (this.config.variableWidth) {
          let e36 = __privateMethod(this, _d_instances, x_fn).call(this, this.target) - this.target;
          this.target += e36 * this.config.snapStrength;
        } else {
          let e36 = Math.round(this.target) - this.target;
          this.target += e36 * this.config.snapStrength;
        }
      }
      if (this.current = c12(this.current, this.target, 1 / this.config.lerpFactor, this.deltaTime), this.config.infinite) {
        if (this.config.variableWidth) {
          let e36 = this.config.vertical ? this.viewport.wrapperHeight / 2 : this.viewport.wrapperWidth / 2, t30 = __privateMethod(this, _d_instances, y_fn).call(this, -this.current + e36), n34 = __privateMethod(this, _d_instances, b_fn).call(this, t30);
          __privateMethod(this, _d_instances, T_fn).call(this, n34), __privateMethod(this, _d_instances, C_fn).call(this);
        } else {
          let e36 = Math.round(-this.current), t30 = this.items.length, n34 = (e36 % t30 + t30) % t30;
          __privateMethod(this, _d_instances, T_fn).call(this, n34), __privateMethod(this, _d_instances, g_fn).call(this);
        }
      } else if (this.config.variableWidth) {
        let e36 = this.config.vertical ? this.viewport.wrapperHeight / 2 : this.viewport.wrapperWidth / 2, t30 = this.config.vertical ? this.viewport.totalHeight : this.viewport.totalWidth, n34 = Math.max(0, Math.min(-this.current + e36, t30));
        __privateMethod(this, _d_instances, T_fn).call(this, __privateMethod(this, _d_instances, b_fn).call(this, n34)), __privateMethod(this, _d_instances, S_fn).call(this);
      } else __privateMethod(this, _d_instances, T_fn).call(this, Math.round(Math.abs(this.current))), __privateMethod(this, _d_instances, h_fn).call(this);
      __privateMethod(this, _d_instances, w_fn).call(this), this.onUpdate?.(this);
    }
    goToNext() {
      if (this.config.variableWidth) {
        let e35 = this.config.infinite ? (this.currentSlide + 1) % this.items.length : Math.min(this.currentSlide + 1, this.items.length - 1);
        this.target = __privateMethod(this, _d_instances, v_fn).call(this, e35);
      } else this.target = this.config.infinite ? Math.round(this.target - 1) : Math.max(this.maxScroll, Math.round(this.target - 1));
    }
    goToPrev() {
      if (this.config.variableWidth) {
        let e35 = this.config.infinite ? (this.currentSlide - 1 + this.items.length) % this.items.length : Math.max(this.currentSlide - 1, 0);
        this.target = __privateMethod(this, _d_instances, v_fn).call(this, e35);
      } else this.target = this.config.infinite ? Math.round(this.target + 1) : Math.min(0, Math.round(this.target + 1));
    }
    goToIndex(e35) {
      if (this.config.variableWidth) {
        let t30 = this.config.infinite ? (e35 % this.items.length + this.items.length) % this.items.length : Math.min(Math.max(e35, 0), this.items.length - 1);
        this.target = __privateMethod(this, _d_instances, v_fn).call(this, t30);
      } else this.target = -e35;
    }
    set snap(e35) {
      this.config.snap = e35;
    }
    getProgress() {
      if (this.config.variableWidth) {
        let e36 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1;
        return (-this.current % e36 + e36) % e36 / e36;
      }
      let e35 = this.items.length;
      return Math.abs(this.current) % e35 / e35;
    }
    destroy() {
      this.kill(), window.removeEventListener(`mousemove`, (e35) => __privateMethod(this, _d_instances, p_fn).call(this, e35)), window.removeEventListener(`mouseup`, () => __privateMethod(this, _d_instances, m_fn).call(this)), window.removeEventListener(`touchmove`, (e35) => {
        let t30 = e35.touches[0];
        __privateMethod(this, _d_instances, p_fn).call(this, t30);
      }), window.removeEventListener(`touchend`, () => __privateMethod(this, _d_instances, m_fn).call(this)), this.wrapper.removeEventListener(`mousedown`, (e35) => __privateMethod(this, _d_instances, f_fn).call(this, e35)), this.wrapper.removeEventListener(`touchstart`, (e35) => {
        let t30 = e35.touches[0];
        __privateMethod(this, _d_instances, f_fn).call(this, t30);
      }), this.resizeTimeout && clearTimeout(this.resizeTimeout), this.virtualScroll && this.config.scrollInput && this.virtualScroll.destroy(), this.observer && this.observer.disconnect();
    }
    get currentSlide() {
      return __privateGet(this, _a);
    }
    kill() {
      __privateSet(this, _r2, false), this.items.forEach((e35) => {
        e35.style.transform = ``;
      }), this.current = 0, this.target = 0, this.speed = 0, __privateSet(this, _e5, 0), this.touchPreviousX = void 0, this.touchPreviousY = void 0, this.isTouching = false;
    }
    init() {
      __privateSet(this, _r2, true), __privateSet(this, _n3, performance.now());
    }
    set paused(e35) {
      __privateSet(this, _i, e35);
    }
    get paused() {
      return __privateGet(this, _i);
    }
    get progress() {
      if (this.config.variableWidth) {
        let e35 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1, t30 = -this.target;
        return this.config.infinite ? (t30 % e35 + e35) % e35 / e35 : Math.max(0, Math.min(t30, e35)) / e35;
      }
      if (this.config.infinite) {
        let e35 = -this.target, t30 = this.items.length;
        return (e35 % t30 + t30) % t30 / (t30 - 1);
      }
      {
        let e35 = Math.abs(this.current), t30 = Math.abs(this.maxScroll);
        return Math.max(0, Math.min(1, e35 / t30));
      }
    }
    resize() {
      if (__privateMethod(this, _d_instances, c_fn).call(this), this.config.variableWidth && !this.config.infinite && this.items.length > 0) {
        let e36 = this.currentSlide, t31 = __privateMethod(this, _d_instances, v_fn).call(this, e36);
        this.target = t31, Math.abs(this.current - this.target) < 1 && (this.current = t31);
      }
      let e35 = __privateGet(this, _r2), t30 = this.isVisible;
      __privateSet(this, _r2, true), this.isVisible = true, this.update(), __privateSet(this, _r2, e35), this.isVisible = t30;
    }
  }, _e5 = new WeakMap(), _t3 = new WeakMap(), _n3 = new WeakMap(), _r2 = new WeakMap(), _i = new WeakMap(), _a = new WeakMap(), _o = new WeakMap(), _d_instances = new WeakSet(), s_fn = function() {
    let e35 = { root: null, rootMargin: `50px`, threshold: 0 };
    this.observer = new IntersectionObserver((e36) => {
      e36.forEach((e37) => {
        this.isVisible = e37.isIntersecting;
      });
    }, e35), this.observer.observe(this.wrapper);
  }, c_fn = function() {
    let e35 = this.items.map((e36) => e36.getBoundingClientRect().width), t30 = this.items.map((e36) => e36.getBoundingClientRect().height), n34 = this.wrapper.clientWidth, r31 = this.wrapper.clientHeight, i31 = e35.reduce((e36, t31) => e36 + t31, 0), a29 = t30.reduce((e36, t31) => e36 + t31, 0), o22 = 0;
    this.itemOffsets = e35.map((e36) => {
      let t31 = o22;
      return o22 += e36, t31;
    }), this.itemWidths = e35;
    let s22 = 0;
    if (this.itemHeightOffsets = t30.map((e36) => {
      let t31 = s22;
      return s22 += e36, t31;
    }), this.itemHeights = t30, this.viewport = { itemWidth: e35[0] ?? 0, wrapperWidth: n34, totalWidth: i31, itemHeight: t30[0] ?? 0, wrapperHeight: r31, totalHeight: a29, vertical: this.config.vertical }, __privateSet(this, _t3, this.config.setOffset(this.viewport)), this.config.variableWidth) this.maxScroll = this.config.vertical ? -(this.viewport.totalHeight - __privateGet(this, _t3)) : -(this.viewport.totalWidth - __privateGet(this, _t3));
    else {
      let e36 = this.config.vertical ? this.viewport.itemHeight || 1 : this.viewport.itemWidth || 1, t31 = this.config.vertical ? this.viewport.totalHeight : this.viewport.totalWidth;
      this.maxScroll = -(t31 - __privateGet(this, _t3)) / e36;
    }
    queueMicrotask(() => {
      this.onResize?.(this);
    });
  }, l_fn = function() {
    this.wrapper.addEventListener(`mousedown`, (e35) => __privateMethod(this, _d_instances, f_fn).call(this, e35)), window.addEventListener(`mousemove`, (e35) => __privateMethod(this, _d_instances, p_fn).call(this, e35)), window.addEventListener(`mouseup`, () => __privateMethod(this, _d_instances, m_fn).call(this)), this.wrapper.addEventListener(`touchstart`, (e35) => {
      let t30 = e35.touches[0];
      this.touchStartY = t30.clientY, this.touchStartX = t30.clientX, this.touchPreviousX = t30.clientX, this.touchPreviousY = t30.clientY, this.scrollDirection = void 0, this.isTouching = true, __privateMethod(this, _d_instances, f_fn).call(this, t30);
    }), window.addEventListener(`touchmove`, (e35) => {
      if (!this.isTouching || __privateGet(this, _i)) return;
      let t30 = e35.touches[0], n34 = Math.abs(t30.clientY - this.touchStartY), r31 = Math.abs(t30.clientX - this.touchStartX);
      !this.scrollDirection && (r31 > 5 || n34 > 5) && (this.scrollDirection = r31 > n34 ? `horizontal` : `vertical`), (this.config.vertical ? this.scrollDirection === `vertical` : this.scrollDirection === `horizontal`) && (e35.preventDefault(), __privateMethod(this, _d_instances, p_fn).call(this, t30), this.config.vertical ? this.touchPreviousY = t30.clientY : this.touchPreviousX = t30.clientX);
    }, { passive: false }), window.addEventListener(`touchend`, () => {
      this.isTouching = false, this.scrollDirection = void 0, this.touchPreviousX = void 0, this.touchPreviousY = void 0, __privateMethod(this, _d_instances, m_fn).call(this);
    }), new ResizeObserver(() => {
      this.resizeTimeout && clearTimeout(this.resizeTimeout), this.resizeTimeout = setTimeout(() => this.resize(), 10);
    }).observe(this.wrapper);
  }, u_fn = function(e35) {
    if (!this.config.infinite) {
      let t30 = this.config.vertical ? this.viewport.itemHeight : this.viewport.itemWidth, n34 = this.config.variableWidth && t30 ? this.config.bounceLimit * t30 : this.config.bounceLimit;
      if (e35 > n34) return n34;
      if (e35 < this.maxScroll - n34) return this.maxScroll - n34;
    }
    return e35;
  }, d_fn = function() {
    this.virtualScroll = new s15.default({ ...this.config.virtualScroll, el: this.wrapper }), this.virtualScroll.on((e35) => {
      if (!this.isDragging && !__privateGet(this, _i)) {
        if (e35.touchDevice) {
          let t31 = Math.abs(e35.deltaY), n35 = Math.abs(e35.deltaX);
          if (t31 < 5 && n35 < 5) return;
          if (this.config.vertical) {
            if (n35 > t31) return;
          } else if (t31 > n35) return;
        }
        let t30 = (this.config.vertical ? this.config.scrollInput ? Math.abs(e35.deltaY) > Math.abs(e35.deltaX) ? e35.deltaY : e35.deltaX : e35.deltaY : this.config.scrollInput ? Math.abs(e35.deltaX) > Math.abs(e35.deltaY) ? e35.deltaX : e35.deltaY : e35.deltaX) * (this.config.variableWidth ? this.config.scrollSensitivity : this.config.scrollSensitivity * 1e-3), n34 = this.target + t30;
        this.config.infinite || (n34 > 0 ? n34 = 0 : n34 < this.maxScroll && (n34 = this.maxScroll)), this.target = __privateMethod(this, _d_instances, u_fn).call(this, n34), this.speed = -t30 * (this.config.variableWidth ? 0.1 : 10);
      }
    });
  }, f_fn = function(e35) {
    __privateGet(this, _i) || (this.isDragging = true, this.dragStart = this.config.vertical ? e35.clientY : e35.clientX, this.dragStartTarget = this.target, this.wrapper.style.cursor = `grabbing`);
  }, p_fn = function(e35) {
    if (!this.isDragging || __privateGet(this, _i)) return;
    let t30 = this.config.vertical ? e35.clientY - this.dragStart : e35.clientX - this.dragStart, n34 = this.config.variableWidth ? 1 : this.config.dragSensitivity, r31 = this.dragStartTarget + t30 * n34;
    if (this.target = __privateMethod(this, _d_instances, u_fn).call(this, r31), `movementX` in e35) {
      let t31 = this.config.vertical ? e35.movementY : e35.movementX;
      this.speed += t31 * 0.01;
    } else {
      let t31 = this.config.vertical ? e35.clientY : e35.clientX, n35 = t31 - (this.config.vertical ? this.touchPreviousY || t31 : this.touchPreviousX || t31);
      this.speed += n35 * 0.01;
    }
  }, m_fn = function() {
    if (this.isDragging = false, this.wrapper.style.cursor = `grab`, this.config.variableWidth) this.config.infinite || (this.target > 0 ? this.target = 0 : this.target < this.maxScroll && (this.target = this.maxScroll)), this.config.snap && (this.target = __privateMethod(this, _d_instances, x_fn).call(this, this.target));
    else if (this.config.infinite) this.config.snap && (this.target = Math.round(this.target));
    else if (this.target > 0) this.target = 0;
    else if (this.target < this.maxScroll) this.target = this.maxScroll;
    else if (this.config.snap) {
      let e35 = Math.round(this.target);
      this.target = Math.min(0, Math.max(this.maxScroll, e35));
    }
  }, h_fn = function() {
    this.parallaxValues = this.items.map((e35, t30) => {
      let n34 = this.config.vertical ? this.current * this.viewport.itemHeight : this.current * this.viewport.itemWidth, r31 = this.config.vertical ? `translateY(${n34}px)` : `translateX(${n34}px)`;
      return e35.style.transform = r31, n34;
    });
  }, g_fn = function() {
    this.parallaxValues = this.items.map((e35, t30) => {
      let n34 = this.current + t30, r31 = (l14(n34, this.items.length) - t30) * (this.config.vertical ? this.viewport.itemHeight : this.viewport.itemWidth), i31 = this.config.vertical ? `translateY(${r31}px)` : `translateX(${r31}px)`;
      return e35.style.transform = i31, l14(n34, this.items.length);
    });
  }, __fn = function(e35) {
    if (this.config.vertical) {
      let t30 = this.itemHeights[e35] ?? this.viewport.itemHeight ?? 0;
      return (this.itemHeightOffsets[e35] ?? 0) + t30 / 2;
    }
    {
      let t30 = this.itemWidths[e35] ?? this.viewport.itemWidth ?? 0;
      return (this.itemOffsets[e35] ?? 0) + t30 / 2;
    }
  }, v_fn = function(e35) {
    let t30 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1, n34 = this.config.vertical ? this.viewport.wrapperHeight / 2 : this.viewport.wrapperWidth / 2, r31 = -(__privateMethod(this, _d_instances, __fn).call(this, e35) - n34);
    if (this.config.infinite) {
      let e36 = Math.round((this.target - r31) / t30);
      r31 += e36 * t30;
    } else r31 = Math.min(0, Math.max(this.maxScroll, r31));
    return r31;
  }, y_fn = function(e35) {
    let t30 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1;
    return (e35 % t30 + t30) % t30;
  }, b_fn = function(e35) {
    let t30 = this.config.vertical ? this.itemHeightOffsets : this.itemOffsets;
    if (!t30.length) return 0;
    let n34 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1, r31 = this.config.infinite ? __privateMethod(this, _d_instances, y_fn).call(this, e35) : Math.max(0, Math.min(e35, n34)), i31 = 0, a29 = 1 / 0;
    return t30.forEach((e36, t31) => {
      let n35 = __privateMethod(this, _d_instances, __fn).call(this, t31), o22 = Math.abs(r31 - n35);
      o22 < a29 && (a29 = o22, i31 = t31);
    }), i31;
  }, x_fn = function(e35) {
    if (!(this.config.vertical ? this.itemHeightOffsets : this.itemOffsets).length) return e35;
    let t30 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1, n34 = this.config.vertical ? this.viewport.wrapperHeight / 2 : this.viewport.wrapperWidth / 2, r31 = this.config.infinite ? __privateMethod(this, _d_instances, y_fn).call(this, -e35 + n34) : Math.max(0, Math.min(-e35 + n34, t30)), i31 = __privateMethod(this, _d_instances, b_fn).call(this, r31);
    return __privateMethod(this, _d_instances, v_fn).call(this, i31);
  }, S_fn = function() {
    this.parallaxValues = this.items.map((e35, t30) => {
      let n34 = this.current, r31 = this.config.vertical ? this.itemHeightOffsets : this.itemOffsets, i31 = this.config.vertical ? `translateY(${n34}px)` : `translateX(${n34}px)`;
      return e35.style.transform = i31, n34 + r31[t30];
    });
  }, C_fn = function() {
    let e35 = this.config.vertical ? this.viewport.totalHeight || 1 : this.viewport.totalWidth || 1;
    this.parallaxValues = this.items.map((t30, n34) => {
      let r31 = (this.config.vertical ? this.itemHeightOffsets : this.itemOffsets)[n34] ?? 0, i31 = l14(this.current + r31, e35) - r31, a29 = this.config.vertical ? `translateY(${i31}px)` : `translateX(${i31}px)`;
      return t30.style.transform = a29, l14(this.current + r31, e35);
    });
  }, w_fn = function() {
    __privateSet(this, _e5, c12(__privateGet(this, _e5), this.speed, 1 / this.config.lerpFactor, this.deltaTime)), this.speed *= this.config.speedDecay;
  }, T_fn = function(e35) {
    __privateGet(this, _a) !== e35 && (__privateSet(this, _o, __privateGet(this, _a)), __privateSet(this, _a, e35), this.onSlideChange?.(__privateGet(this, _a), __privateGet(this, _o)));
  }, _a2);
  var f12 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var p14 = (e35) => String(e35).padStart(2, `0`);
  document.querySelectorAll(`[data-slider]`).forEach((n34) => {
    let r31 = n34.closest(`[data-slider-root]`) ?? n34.parentElement, i31 = r31.querySelector(`[data-slider-count]`), a29 = n34.children.length, o22 = r31.querySelector(`[data-slider-caption]`), s22 = Array.from(n34.children).map((e35) => e35.dataset.caption ?? ``), c17 = n34.dataset.infinite !== `false`;
    if (c17 && a29 > 0) {
      let e35 = Array.from(n34.children), r32 = e35[0].getBoundingClientRect().width || innerWidth / 2, i32 = Math.max(innerWidth, screen.width || 0, 1920) * 2.5;
      for (; n34.children.length * r32 < i32 && n34.children.length < a29 * 12; ) e35.forEach((e36) => {
        let t30 = e36.cloneNode(true);
        t30.setAttribute(`aria-hidden`, `true`), t30.setAttribute(`inert`, ``), n34.append(t30);
      });
      n34.dataset.reveal === `cards` && n34.children.length > a29 && m13(Array.from(n34.children).map((e36) => e36.firstElementChild ?? e36));
    }
    n34.querySelectorAll(`img[loading=lazy]`).forEach((e35) => {
      e35.loading = `eager`;
    });
    let l18 = -1, u17 = (t30) => {
      let r32 = (t30 % a29 + a29) % a29 + 1;
      if (o22 && s22[r32 - 1] && o22.textContent !== s22[r32 - 1] && oi.to(o22, { opacity: 0, duration: f12 ? 0 : 0.15, ease: `power2.out`, overwrite: true, onComplete: () => {
        o22.textContent = s22[r32 - 1], oi.to(o22, { opacity: 1, duration: f12 ? 0 : 0.35, ease: `power2.out` });
      } }), i31 && (i31.textContent = `${p14(r32)}/${p14(a29)}`), r32 - 1 !== l18) {
        let e35 = l18 < 0 ? 0 : (r32 - 1 - l18 + a29 + a29 / 2) % a29 - a29 / 2;
        l18 = r32 - 1, n34.dispatchEvent(new CustomEvent(`slider:change`, { detail: { index: r32 - 1, dir: e35 < 0 ? -1 : 1 } }));
      }
    }, m14 = n34.dataset.drag === `false`, h12 = new d13(n34, { infinite: c17, snap: true, lerpFactor: f12 ? 1 : 0.08, snapStrength: 0.1, onSlideChange: (e35) => u17(e35), onUpdate: (e35) => {
      e35.items.forEach((t30, n35) => {
        let r32 = n35 + e35.current, i32 = e35.items.length, a30 = (r32 % i32 + i32 + i32 / 2) % i32 - i32 / 2;
        t30.style.setProperty(`--slide-dist`, Math.min(1, Math.abs(a30)).toFixed(3));
      });
    } });
    u17(0), (m14 || n34.dataset.center === `true`) && (n34.style.cursor = ``);
    let g14 = () => {
      h12.config.dragSensitivity = 1 / (h12.viewport.itemWidth || 1);
    };
    g14(), addEventListener(`resize`, g14), oi.ticker.add(() => h12.update());
    let _13 = 0, v13 = 0, y13 = 0, b13 = false;
    n34.addEventListener(`pointerdown`, (e35) => {
      b13 = true, _13 = e35.clientX, v13 = y13 = h12.target;
    }), addEventListener(`pointermove`, () => {
      b13 && (y13 = h12.target);
    }), addEventListener(`pointerup`, (e35) => {
      b13 && (b13 = false, requestAnimationFrame(() => {
        Math.abs(e35.clientX - _13) > 60 && Math.round(h12.target) === Math.round(v13) && y13 !== v13 && (h12.target = Math.round(v13) + Math.sign(y13 - v13));
      }));
    });
    let x13 = r31.querySelector(`[data-slider-prev]`), S12 = r31.querySelector(`[data-slider-next]`);
    if (n34.addEventListener(`slider:next`, () => h12.goToNext()), n34.addEventListener(`slider:go`, (e35) => h12.goToIndex(e35.detail)), n34.addEventListener(`focusin`, (e35) => {
      let t30 = Array.from(n34.children).find((t31) => t31.contains(e35.target));
      if (!t30) return;
      for (let e36 = n34; e36 && e36 !== r31.parentElement; e36 = e36.parentElement) e36.scrollLeft && (e36.scrollLeft = 0);
      let i32 = t30.getBoundingClientRect(), o23 = r31.getBoundingClientRect();
      (i32.left < Math.max(0, o23.left) - 1 || i32.right > Math.min(innerWidth, o23.right) + 1) && h12.goToIndex(Array.from(n34.children).indexOf(t30) % a29);
    }), x13?.addEventListener(`click`, () => h12.goToPrev()), S12?.addEventListener(`click`, () => h12.goToNext()), !c17 && (x13 || S12) && oi.ticker.add(() => {
      let e35 = h12.target > -0.02, t30 = h12.target < h12.maxScroll + 0.02;
      x13 && x13.disabled !== e35 && (x13.disabled = e35), S12 && S12.disabled !== t30 && (S12.disabled = t30);
    }), m14 || n34.dataset.center === `true`) {
      let t30 = Array.from(n34.children), r32 = t30.length, i32 = n34.parentElement, a30 = () => {
        let e35 = (-Math.round(h12.target) % r32 + r32) % r32;
        t30.forEach((t31, n35) => t31.classList.toggle(`is-active`, n35 === e35));
      };
      a30(), oi.ticker.add(a30), m14 && i32.addEventListener(`mousedown`, (e35) => {
        n34.contains(e35.target) && e35.stopPropagation();
      }, true);
      let o23 = (e35, n35) => t30.findIndex((t31) => {
        let r33 = t31.getBoundingClientRect();
        return e35 >= r33.left && e35 < r33.right && n35 >= r33.top && n35 < r33.bottom;
      }), s23 = 0;
      i32.addEventListener(`pointerdown`, (e35) => {
        s23 = e35.clientX;
      }), i32.addEventListener(`mousemove`, (e35) => {
        let n35 = o23(e35.clientX, e35.clientY);
        i32.style.cursor = n35 >= 0 && !t30[n35].classList.contains(`is-active`) ? `pointer` : ``;
      }), i32.addEventListener(`click`, (e35) => {
        if (Math.abs(e35.clientX - s23) > 6 || e35.target.closest(`button, a`)) return;
        let n35 = o23(e35.clientX, e35.clientY);
        if (n35 < 0 || t30[n35].classList.contains(`is-active`)) return;
        i32.style.cursor = ``;
        let a31 = Math.round(h12.target), c18 = ((a31 + n35) % r32 + r32) % r32;
        c18 > r32 / 2 && (c18 -= r32), h12.target = a31 - c18;
      }), !x13 && !S12 && r32 > 1 && (n34.tabIndex = 0, n34.addEventListener(`keydown`, (e35) => {
        e35.key === `ArrowLeft` && (e35.preventDefault(), h12.goToPrev()), e35.key === `ArrowRight` && (e35.preventDefault(), h12.goToNext());
      }));
    }
    let C12 = 0;
    n34.addEventListener(`pointerdown`, (e35) => {
      C12 = e35.clientX;
    }), n34.addEventListener(`click`, (e35) => {
      e35.detail > 0 && Math.abs(e35.clientX - C12) > 6 && (e35.preventDefault(), e35.stopPropagation());
    }, true);
  });

  // HomeProducts.astro_astro_type_script_index_0_lang.DG2qerjp.js
  var t20 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  document.querySelectorAll(`[data-home-products]`).forEach((n34) => {
    let r31 = Array.from(n34.querySelectorAll(`[data-hp-tab]`)), i31 = Array.from(n34.querySelectorAll(`[data-hp-panel]`));
    n34.querySelectorAll(`[data-hp-copy] a, [data-hp-copy] button`).forEach((e35) => e35.setAttribute(`tabindex`, `-1`));
    let a29 = n34.querySelector(`[data-hp-counts]`), o22 = a29 ? JSON.parse(a29.dataset.hpCounts || `{}`) : {}, s22 = a29 ? Array.from(a29.querySelectorAll(`.odo-strip`)) : [];
    s22.forEach((t30) => {
      let n35 = (t30.style.transform || t30.dataset.odo || ``).match(/-?[\d.]+/);
      t30.style.transform = ``, oi.set(t30, { yPercent: n35 ? Number(n35[0]) : 0 });
    }), a29?.querySelectorAll(`.hp-line`).forEach((t30) => oi.set(t30, { yPercent: t30.classList.contains(`is-active`) ? 0 : 110, opacity: 1 }));
    let c17 = (n35) => {
      if (!a29) return;
      let r32 = t20 ? 0 : 0.7;
      a29.querySelectorAll(`.hp-mask`).forEach((t30) => {
        let i33 = Array.from(t30.querySelectorAll(`.hp-line`)), a30 = i33.find((e35) => e35.classList.contains(`is-active`)), o23 = i33.find((e35) => e35.dataset.hpLine === n35);
        o23 && o23 !== a30 && (a30 && (a30.classList.remove(`is-active`), oi.to(a30, { yPercent: -110, duration: r32, ease: `power3.inOut`, overwrite: true })), o23.classList.add(`is-active`), oi.fromTo(o23, { yPercent: 110 }, { yPercent: 0, duration: r32, ease: `power3.inOut`, overwrite: true }));
      });
      let i32 = o22[n35] ?? `000`;
      s22.forEach((n36, r33) => {
        oi.to(n36, { yPercent: -Number(i32[r33] ?? 0) * 10, duration: t20 ? 0 : 0.9, ease: `power4.inOut`, delay: t20 ? 0 : r33 * 0.06, overwrite: true });
      });
    }, l18 = (e35) => {
      r31.forEach((t30) => {
        let n35 = t30.dataset.hpTab === e35;
        t30.classList.toggle(`is-active`, n35), t30.setAttribute(`aria-selected`, String(n35));
      }), c17(e35), i31.forEach((t30) => {
        let n35 = t30.dataset.hpPanel === e35;
        t30.classList.toggle(`is-active`, n35), t30.toggleAttribute(`inert`, !n35);
      });
    };
    r31.forEach((e35, t30) => {
      e35.addEventListener(`click`, (t31) => {
        e35.classList.contains(`is-active`) || (t31.preventDefault(), l18(e35.dataset.hpTab));
      }), e35.addEventListener(`keydown`, (e36) => {
        if (e36.key !== `ArrowRight` && e36.key !== `ArrowLeft`) return;
        e36.preventDefault();
        let n35 = r31[(t30 + (e36.key === `ArrowRight` ? 1 : -1) + r31.length) % r31.length];
        l18(n35.dataset.hpTab), n35.focus();
      });
    });
  });

  // MockupDrift.astro_astro_type_script_index_0_lang.JkKcWt-F.js
  oi.registerPlugin($4);
  var n24 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var r21 = (e35) => e35 - Math.floor(e35);
  document.querySelectorAll(`[data-drift]`).forEach((i31) => {
    let a29 = i31.querySelector(`[data-drift-pin]`), o22 = i31.querySelector(`[data-drift-fade]`), s22 = Array.from(i31.querySelectorAll(`[data-drift-item]`)), c17 = s22.length;
    if (!c17) return;
    let l18 = Math.ceil(c17 / 2), u17 = s22.map((e35, t30) => ({ phase: r21(t30 % l18 * 0.43 + Math.floor(t30 / l18) * 0.5), angle: t30 % l18 / l18 * Math.PI * 2 + Math.floor(t30 / l18) * 0.6, near: 0.6 + t30 % 3 * 0.12, tilt: (t30 % 2 ? 1 : -1) * (2 + t30 % 4) })), d16 = a29.clientWidth, f15 = a29.clientHeight, p15 = () => {
      d16 = a29.clientWidth, f15 = a29.clientHeight;
    };
    addEventListener(`resize`, p15);
    let m14 = 0, h12 = 0, g14 = 0, _13 = false, v13 = () => {
      let e35 = d16 < 768, t30 = d16 / 2 * (e35 ? 1.3 : 1), n34 = f15 / 2;
      s22.forEach((e36, i32) => {
        let a30 = u17[i32], o23 = r21(a30.phase + h12 + g14), s23 = Math.abs(2 * o23 - 1), c18 = a30.near + (1.4 - a30.near) * s23 ** 2.2, l19 = a30.angle + (o23 - 0.5) * Math.PI * 1.15, d17 = Math.cos(l19) * c18 * t30, f16 = Math.sin(l19) * c18 * n34, p16 = 0.72 + 0.4 * Math.min(1, (c18 - a30.near) / 0.8);
        e36.style.transform = `translate3d(${d17.toFixed(1)}px, ${f16.toFixed(1)}px, 0) rotate(${(a30.tilt * (1 - s23)).toFixed(2)}deg) scale(${p16.toFixed(3)})`, e36.style.zIndex = String(Math.round(p16 * 10));
      });
    };
    n24 ? (v13(), s22.forEach((e35) => e35.style.opacity = `1`)) : ($4.create({ trigger: i31, start: `top 80%`, once: true, onEnter: () => oi.to(s22, { opacity: 1, duration: 1.2, ease: `power2.out`, stagger: 0.04 }) }), $4.create({ trigger: a29, start: `top top`, end: `+=78%`, pin: true, invalidateOnRefresh: true, onRefresh: p15, onUpdate: (e35) => {
      m14 = e35.progress * 0.65;
    } }), o22 && oi.fromTo(o22, { opacity: 0 }, { opacity: 1, ease: `none`, scrollTrigger: { trigger: i31, start: () => `top+=${innerHeight * 0.5} top`, end: () => `top+=${innerHeight * 0.76} top`, scrub: true, invalidateOnRefresh: true } }), $4.create({ trigger: i31, start: `top bottom`, end: `top top`, onUpdate: (e35) => {
      e35.progress < 1 && (m14 = (e35.progress - 1) * 0.25);
    } }), new IntersectionObserver(([e35]) => {
      _13 = e35.isIntersecting;
    }, { rootMargin: `10% 0px` }).observe(a29), oi.ticker.add((e35, t30) => {
      _13 && (g14 += t30 / 1e3 * 6e-3, h12 += (m14 - h12) * Math.min(1, t30 / 1e3 * 3.2), v13());
    }), v13(), addEventListener(`load`, () => $4.refresh()));
  });

  // details-art.CBAlSB9q.js
  var t21 = (e35, t30) => e35.querySelector(t30);
  var n25 = (e35, t30) => [...e35.querySelectorAll(t30)];
  function r22(e35, t30, n34 = 0.5, r31 = 0.5) {
    let i31 = e35.getBoundingClientRect(), a29 = t30.getBoundingClientRect();
    return { x: a29.left - i31.left + a29.width * n34, y: a29.top - i31.top + a29.height * r31, w: a29.width, h: a29.height, l: a29.left - i31.left, t: a29.top - i31.top };
  }
  function i22(e35) {
    let n34 = t21(e35, `.da-cursor`), r31 = () => ({ x: n34.getBoundingClientRect().width * 0.2, y: n34.getBoundingClientRect().height * 0.12 });
    return { el: n34, to: (e36) => ({ x: e36.x - r31().x, y: e36.y - r31().y }), click: (e36, t30) => e36.to(n34, { scale: 0.82, duration: 0.12, yoyo: true, repeat: 1, ease: `power2.inOut`, transformOrigin: `20% 12%` }, t30) };
  }
  var a20 = (e35) => Math.round(e35).toLocaleString(`en-US`);
  var o15 = { "grid-build"(a29) {
    let o22 = t21(a29, `[data-gb-grid]`), s22 = t21(a29, `[data-gb-frame]`), c17 = t21(a29, `[data-gb-name]`), l18 = n25(a29, `[data-gb-ch]`), u17 = t21(a29, `[data-gb-caret]`), d16 = n25(a29, `[data-gb-line]`), f15 = t21(a29, `[data-gb-btn]`), p15 = n25(a29, `[data-gb-red]`), m14 = t21(a29, `[data-gb-off]`), h12 = t21(a29, `[data-gb-handle]`), g14 = p15.map((e35) => t21(e35, `em`)), _13 = i22(a29), v13 = a29.clientWidth * 0.0125, y13 = s22.offsetWidth, b13 = s22.getBoundingClientRect(), x13 = t21(s22, `.gb-title`).getBoundingClientRect(), S12 = t21(s22, `.gb-text`).getBoundingClientRect(), C12 = f15.getBoundingClientRect();
    oi.set(p15[0], { top: x13.bottom - b13.top, height: S12.top - x13.bottom }), oi.set(p15[1], { top: S12.bottom - b13.top, height: C12.top - S12.bottom }), oi.set(p15[2], { top: C12.top - b13.top + C12.height / 2 }), oi.set(m14, { left: C12.right - b13.left + v13, top: C12.top - b13.top - v13 * 0.6 });
    let w10 = oi.timeline({ repeat: -1, defaults: { ease: `power3.out` } });
    return w10.call(() => {
      g14.forEach((e35) => e35.textContent = e35.dataset.gbVal), m14.textContent = `13`, s22.classList.remove(`is-sel`);
    }), w10.set(_13.el, { opacity: 0, ..._13.to({ x: a29.clientWidth * 0.9, y: a29.clientHeight * 0.9 }) }), w10.fromTo(o22, { clipPath: `inset(100% 0% 0% 0%)` }, { clipPath: `inset(0% 0% 0% 0%)`, duration: 1.1, ease: `power2.inOut` }), w10.fromTo(s22, { opacity: 0 }, { opacity: 1, duration: 0.5 }, `-=0.3`), w10.set(u17, { opacity: 1 }), w10.to(l18, { opacity: 1, duration: 0.01, stagger: 0.04 }), w10.set(u17, { opacity: 0 }, `+=0.15`), w10.fromTo(d16, { width: 0 }, { width: (e35) => e35 ? `70%` : `100%`, duration: 0.6, stagger: 0.1 }, `-=0.1`), w10.fromTo(f15, { opacity: 0, y: -v13 * 3, x: v13 * 0.75 }, { opacity: 1, y: v13 * 0.75, duration: 0.5 }, `-=0.2`), w10.fromTo(m14, { opacity: 0 }, { opacity: 1, duration: 0.2 }), w10.to(f15, { x: 0, y: 0, duration: 0.45, ease: `power2.inOut` }, `+=0.35`), w10.call(() => {
      m14.textContent = `12`;
    }, void 0, `<0.2`), w10.to(m14, { opacity: 0, duration: 0.3 }, `+=0.35`), w10.fromTo(p15, { opacity: 0 }, { opacity: 1, duration: 0.3, stagger: 0.12 }), w10.to(g14, { opacity: 0, duration: 0.15 }, `+=0.6`), w10.call(() => {
      g14[0].textContent = `Gap 04`, g14[1].textContent = `Gap 06`, g14[2].textContent = `Gap 06`;
    }), w10.to(g14, { opacity: 1, duration: 0.2 }), w10.to(p15, { opacity: 0, duration: 0.2 }, `+=0.7`), w10.call(() => s22.classList.add(`is-sel`)), w10.to([c17, h12], { opacity: 1, duration: 0.2 }), w10.set(_13.el, _13.to(r22(a29, h12)), `<`), w10.fromTo(_13.el, { opacity: 0 }, { opacity: 1, duration: 0.2 }, `<`), w10.to(s22, { width: y13 - v13 * 16, duration: 1, ease: `power2.inOut` }, `+=0.15`), w10.to(_13.el, { x: `-=${v13 * 8}`, duration: 1, ease: `power2.inOut` }, `<`), w10.to(s22, { width: y13, duration: 0.9, ease: `power2.inOut` }, `+=0.5`), w10.to(_13.el, { x: `+=${v13 * 8}`, duration: 0.9, ease: `power2.inOut` }, `<`), w10.to([_13.el, c17, h12], { opacity: 0, duration: 0.3 }, `+=0.3`), w10.call(() => s22.classList.remove(`is-sel`)), w10.to(s22, { opacity: 0, duration: 0.5, ease: `power2.inOut` }, `+=0.4`), w10.to(o22, { clipPath: `inset(100% 0% 0% 0%)`, duration: 0.8, ease: `power2.inOut` }, `<0.1`), w10;
  }, "grid-vars"(a29) {
    let o22 = i22(a29), s22 = (e35) => t21(a29, `[data-gv-row="${e35}"]`), c17 = t21(a29, `[data-gv-num="gap"]`), l18 = t21(a29, `[data-gv-num="rad"]`), u17 = t21(a29, `[data-gv-hex]`), d16 = { g: 16, r: 20 }, f15 = oi.timeline({ repeat: -1, defaults: { ease: `power3.inOut` } });
    f15.call(() => {
      c17.textContent = `16`, l18.textContent = `20`, u17.textContent = `EFF1F0`, n25(a29, `.gv-row`).forEach((e35) => e35.classList.remove(`is-on`));
    }), f15.set(a29, { "--g": `1.6cqw`, "--r": `2cqw`, "--s": `#eff1f0` }), f15.set(o22.el, o22.to({ x: a29.clientWidth * 0.5, y: a29.clientHeight * 0.92 }));
    let p15 = (e35, n34) => {
      f15.to(o22.el, { ...o22.to(r22(a29, t21(s22(e35), `.gv-val`))), duration: 0.8 }, `+=0.3`), o22.click(f15), f15.call(() => s22(e35).classList.add(`is-on`)), n34(f15), f15.call(() => s22(e35).classList.remove(`is-on`), void 0, `+=0.5`);
    };
    return p15(`gap`, (e35) => {
      e35.to(d16, { g: 24, duration: 0.8, ease: `power2.inOut`, onUpdate: () => {
        c17.textContent = String(Math.round(d16.g / 4) * 4);
      } }, `+=0.1`), e35.to(a29, { "--g": `2.4cqw`, duration: 0.8, ease: `power2.inOut` }, `<`);
    }), p15(`rad`, (e35) => {
      e35.to(d16, { r: 8, duration: 0.8, ease: `power2.inOut`, onUpdate: () => {
        l18.textContent = String(Math.round(d16.r / 4) * 4);
      } }, `+=0.1`), e35.to(a29, { "--r": `0.8cqw`, duration: 0.8, ease: `power2.inOut` }, `<`);
    }), p15(`sur`, (e35) => {
      e35.call(() => {
        u17.textContent = `DCFF02`;
      }, void 0, `+=0.1`), e35.to(a29, { "--s": `#dcff02`, duration: 0.6, ease: `power2.inOut` }, `<`);
    }), f15.to(o22.el, { ...o22.to(r22(a29, t21(a29, `.gv-mode`))), duration: 0.7 }, `+=0.6`), o22.click(f15), f15.call(() => {
      u17.textContent = `EFF1F0`;
    }), f15.to(d16, { g: 16, r: 20, duration: 0.8, ease: `power2.inOut`, onUpdate: () => {
      c17.textContent = String(Math.round(d16.g / 4) * 4), l18.textContent = String(Math.round(d16.r / 4) * 4);
    } }), f15.to(a29, { "--g": `1.6cqw`, "--r": `2cqw`, "--s": `#eff1f0`, duration: 0.8, ease: `power2.inOut` }, `<`), f15.to(o22.el, { ...o22.to({ x: a29.clientWidth * 0.5, y: a29.clientHeight * 0.92 }), duration: 0.8 }, `+=0.4`), f15;
  }, "files-blind"(a29) {
    let o22 = i22(a29), s22 = t21(a29, `[data-fb-dark]`), c17 = t21(a29, `[data-fb-sel]`), l18 = t21(a29, `[data-fb-tag]`), u17 = oi.timeline({ repeat: -1, defaults: { ease: `power3.inOut` } }), d16 = { x: a29.clientWidth * 0.62, y: a29.clientHeight * 0.9 };
    u17.call(() => n25(a29, `.fb-row`).forEach((e35) => e35.classList.remove(`is-sel`))), u17.set(c17, { opacity: 0 }), u17.set(o22.el, o22.to(d16));
    let f15 = (e35, i31) => {
      let d17 = t21(a29, `[data-fb-row="${e35}"]`), f16 = t21(a29, `[data-fb-el="${e35}"]`), p15 = r22(c17.parentElement, f16);
      u17.to(s22, { opacity: 0.9, duration: 0.6, ease: `power2.inOut` }, `+=0.4`), u17.to(o22.el, { ...o22.to(r22(a29, d17, 0.35)), duration: 1.1 }, `+=0.2`), o22.click(u17), u17.call(() => {
        n25(a29, `.fb-row`).forEach((e36) => e36.classList.toggle(`is-sel`, e36 === d17)), l18.textContent = i31;
      }), u17.set(c17, { left: p15.l, top: p15.t, width: p15.w, height: p15.h }), u17.to(s22, { opacity: 0, duration: 0.5, ease: `power2.out` }), u17.fromTo(c17, { opacity: 0 }, { opacity: 1, duration: 0.25 }, `<0.1`), u17.to(c17, { opacity: 0, duration: 0.3 }, `+=1.3`);
    };
    return f15(`btn`, `Button \xB7 120 \xD7 40`), f15(`title`, `Title \xB7 Display`), u17.call(() => n25(a29, `.fb-row`).forEach((e35) => e35.classList.remove(`is-sel`))), u17.to(o22.el, { ...o22.to(d16), duration: 0.8 }, `+=0.1`), u17;
  }, "files-same"(t30) {
    let i31 = n25(t30, `[data-fs-file]`), a29 = i31.map((e35) => n25(e35, `[data-fs-page]`)), o22 = oi.timeline({ repeat: -1, defaults: { ease: `power3.out` } });
    o22.fromTo(i31, { opacity: 0, y: t30.clientHeight * 0.04 }, { opacity: 1, y: 0, duration: 0.7, stagger: 0.1 }), a29[0].forEach((e35, t31) => {
      let n34 = a29.map((e36) => e36[t31]);
      o22.to(n34, { "--on": 1, duration: 0.18, ease: `power1.out` }, t31 === 0 ? `+=0.2` : `-=0.06`), o22.to(n34, { "--on": 0, duration: 0.3, ease: `power1.in` }, `+=0.04`);
    });
    let s22 = r22(t30, i31[1]).x;
    return o22.to(i31, { x: (e35) => s22 - r22(t30, i31[e35]).x, duration: 1, ease: `power3.inOut` }, `+=0.2`), o22.to(i31.filter((e35, t31) => t31 !== 1), { opacity: 0, duration: 0.2 }, `-=0.15`), o22.to(i31, { x: 0, opacity: 1, duration: 1, ease: `power3.inOut` }, `+=1`), o22.to(i31, { opacity: 0, y: -t30.clientHeight * 0.03, duration: 0.5, stagger: 0.06, ease: `power2.in` }, `+=1`), o22;
  }, "calm-order"(i31) {
    let a29 = n25(i31, `[data-co="title"]`), o22 = n25(i31, `[data-co="text"]`), s22 = t21(i31, `[data-co="btn"]`), c17 = t21(i31, `[data-co="pic"]`), l18 = n25(i31, `[data-co-bar]`), u17 = t21(i31, `[data-co-head]`), d16 = t21(i31, `.co-curve svg path`), f15 = r22(i31, t21(i31, `.co-lane`)), p15 = r22(i31, t21(i31, `.co-tracks`)), m14 = f15.l - p15.l, h12 = 2.6, g14 = oi.timeline({ repeat: -1, defaults: { ease: `power3.out` } });
    return g14.set(u17, { left: m14, opacity: 1 }), g14.fromTo(d16, { "--dash": 60 }, { "--dash": 0, duration: 0.8, ease: `power2.inOut` }), g14.addLabel(`go`, `+=0.2`), g14.to(u17, { left: m14 + f15.w, duration: h12, ease: `none` }, `go`), g14.fromTo(l18, { scaleX: 0 }, { scaleX: 1, duration: h12 * 0.4, ease: `none`, stagger: h12 * 0.12 }, `go`), g14.fromTo(a29, { yPercent: 110 }, { yPercent: 0, duration: h12 * 0.4, stagger: 0.08 }, `go`), o22.length && g14.fromTo(o22, { opacity: 0, y: `1cqw` }, { opacity: 1, y: 0, duration: h12 * 0.4 }, `go+=${h12 * 0.12}`), g14.fromTo(s22, { opacity: 0, y: `1cqw` }, { opacity: 1, y: 0, duration: h12 * 0.4 }, `go+=${h12 * (o22.length ? 0.24 : 0.12)}`), g14.fromTo(c17, { clipPath: `inset(100% 0% 0% 0%)` }, { clipPath: `inset(0% 0% 0% 0%)`, duration: h12 * 0.4 }, `go+=${h12 * (o22.length ? 0.36 : 0.24)}`), g14.fromTo(t21(c17, `span`), { scale: 1.15 }, { scale: 1, duration: h12 * 0.6 }, `<`), g14.to(u17, { opacity: 0, duration: 0.3 }), g14.to([a29, o22, s22, c17].flat(), { opacity: 0, duration: 0.6, ease: `power2.inOut` }, `+=1.6`), g14.to(l18, { scaleX: 0, transformOrigin: `right`, duration: 0.6, ease: `power2.inOut` }, `<`), g14.set([a29, o22, s22, c17].flat(), { opacity: 1 }), g14.set(l18, { transformOrigin: `left` }), g14;
  }, "calm-off"(a29) {
    let o22 = i22(a29), s22 = t21(a29, `[data-cf-btn]`), c17 = t21(a29, `[data-cf-face]`), l18 = n25(a29, `[data-cf-row]`), u17 = n25(a29, `[data-cf-fx]`), d16 = oi.timeline({ repeat: -1, defaults: { ease: `power3.inOut` } }), f15 = { x: a29.clientWidth * 0.5, y: a29.clientHeight * 0.92 };
    return d16.call(() => {
      l18.forEach((e35) => e35.classList.remove(`is-off`)), s22.classList.add(`is-wobble`), c17.classList.remove(`is-hover`);
    }), d16.set(u17, { opacity: 1 }), d16.set(o22.el, o22.to(f15)), l18.forEach((e35, n34) => {
      d16.to(o22.el, { ...o22.to(r22(a29, t21(e35, `[data-cf-eye]`))), duration: n34 ? 0.45 : 0.8 }, n34 ? `+=0.15` : `+=0.6`), o22.click(d16), d16.call(() => {
        e35.classList.add(`is-off`), n34 === 4 && s22.classList.remove(`is-wobble`);
      }), u17[n34] && d16.to(u17.filter((e36) => e36.dataset.cfFx === String(n34)), { opacity: 0, duration: 0.4, ease: `power2.out` }, `<`);
    }), d16.to(o22.el, { ...o22.to(r22(a29, c17, 0.7, 0.6)), duration: 0.9 }, `+=0.5`), d16.call(() => c17.classList.add(`is-hover`)), d16.call(() => c17.classList.remove(`is-hover`), void 0, `+=1`), d16.to(o22.el, { ...o22.to(f15), duration: 0.8 }, `+=0.1`), d16.to(u17, { opacity: 1, duration: 0.5 }, `+=0.3`), d16.call(() => {
      l18.forEach((e35) => e35.classList.remove(`is-off`)), s22.classList.add(`is-wobble`);
    }, void 0, `<`), d16;
  }, "count-wall"(n34) {
    let r31 = t21(n34, `[data-cw-canvas]`), i31 = t21(n34, `[data-cw-num]`), o22 = r31.getContext(`2d`), s22 = Math.min(2, window.devicePixelRatio || 1), c17 = n34.clientWidth, l18 = n34.clientHeight;
    r31.width = c17 * s22, r31.height = l18 * s22;
    let u17 = 3700, d16 = c17 * 0.04, f15 = (c17 - d16 * 2) / 74, p15 = (l18 - d16 * 2) / 50, m14 = f15 * 0.8, h12 = p15 * 0.8, g14 = 108, _13 = () => (g14 = g14 * 16807 % 2147483647) / 2147483647, v13 = Array.from({ length: u17 }, () => {
      let e35 = _13();
      return e35 < 0.05 ? 2 : e35 < 0.12 ? 1 : e35 < 0.45 ? 3 : 0;
    }), y13 = new Float32Array(u17), b13 = Array.from({ length: u17 }, (e35, t30) => t30), x13 = b13.map((e35) => Math.hypot(e35 % 74 - 37, (Math.floor(e35 / 74) - 25) * 1.1) + _13() * 1.5);
    b13.sort((e35, t30) => x13[e35] - x13[t30]).forEach((e35, t30) => y13[e35] = t30);
    let S12 = c17 * 0.46 / m14, C12 = d16 + 37 * f15 + f15 / 2, w10 = d16 + 25 * p15 + p15 / 2, T10 = (e35, t30, n35, r32, i32, a29) => {
      o22.fillStyle = a29, o22.beginPath(), o22.roundRect(e35, t30, n35, r32, Math.min(i32, n35 / 2, r32 / 2)), o22.fill();
    }, E9 = (e35, t30) => {
      let n35 = e35, r32 = S12 ** (1 - n35), i32 = C12 + (c17 / 2 - C12) * n35, a29 = w10 + (l18 / 2 - w10) * n35;
      o22.setTransform(s22, 0, 0, s22, 0, 0), o22.clearRect(0, 0, c17, l18), o22.translate(c17 / 2, l18 / 2), o22.scale(r32, r32), o22.translate(-i32, -a29);
      let u18 = i32 - c17 / 2 / r32, g15 = i32 + c17 / 2 / r32, _14 = a29 - l18 / 2 / r32, b14 = a29 + l18 / 2 / r32, x14 = Math.max(0, Math.floor((u18 - d16) / f15)), E10 = Math.min(73, Math.ceil((g15 - d16) / f15)), D10 = Math.max(0, Math.floor((_14 - d16) / p15)), O9 = Math.min(49, Math.ceil((b14 - d16) / p15)), k9 = m14 * r32 > 50, A8 = 1 / r32;
      for (let e36 = D10; e36 <= O9; e36++) for (let n36 = x14; n36 <= E10; n36++) {
        let r33 = e36 * 74 + n36;
        if (y13[r33] >= t30) continue;
        let i33 = d16 + n36 * f15 + (f15 - m14) / 2, a30 = d16 + e36 * p15 + (p15 - h12) / 2, o23 = v13[r33], s23 = o23 === 2, c18 = s23 ? `#ffffff` : `#0a0a0c`;
        if (T10(i33, a30, m14, h12, m14 * 0.06, s23 ? `#0a0a0c` : `#ffffff`), k9) {
          let e37 = m14 * 0.08, t31 = m14 - e37 * 2;
          T10(i33 + e37, a30 + e37, t31 * 0.28, h12 * 0.025, h12 * 0.0125, c18);
          let n37 = a30 + h12 * (o23 === 3 ? 0.14 : 0.12);
          T10(i33 + e37, n37, t31 * 0.9, h12 * 0.05, h12 * 0.02, c18), T10(i33 + e37, n37 + h12 * 0.07, t31 * 0.55, h12 * 0.05, h12 * 0.02, c18), T10(i33 + e37, n37 + h12 * 0.15, t31 * 0.3, h12 * 0.04, h12 * 0.02, s23 ? `#ffffff` : `#0a0a0c`), T10(i33 + e37, a30 + h12 * 0.42, t31, h12 * 0.5, m14 * 0.03, o23 === 1 ? `#dcff02` : s23 ? `#333333` : `#e3e7e5`);
        } else T10(i33 + m14 * 0.12, a30 + h12 * 0.14, m14 * 0.6, Math.max(h12 * 0.08, A8), 0, c18), T10(i33 + m14 * 0.12, a30 + h12 * 0.42, m14 * 0.76, h12 * 0.46, 0, o23 === 1 ? `#dcff02` : s23 ? `#333333` : `#e3e7e5`);
      }
    }, D9 = { p: 0, n: 1 }, O8 = () => {
      E9(D9.p, D9.n), i31.textContent = a20(D9.n);
    }, k8 = oi.timeline({ repeat: -1, onUpdate: O8 });
    return k8.set(r31, { opacity: 1 }), k8.set(D9, { p: 0, n: 1 }), k8.to(D9, { p: 1, duration: 4.6, ease: `power2.inOut` }, `+=0.8`), k8.to(D9, { n: u17, duration: 4.4, ease: `power2.in` }, `<0.2`), k8.to(r31, { opacity: 0, duration: 0.6, ease: `power2.inOut` }, `+=1.6`), O8(), k8;
  }, "count-strip"(r31) {
    let i31 = t21(r31, `[data-cs-track]`), o22 = t21(r31, `[data-cs-num]`), s22 = t21(r31, `[data-cs-age]`), c17 = n25(i31, `.cs-page`), l18 = r31.clientWidth * 0.43, u17 = c17.length - 1, d16 = { i: 0 }, f15 = () => {
      oi.set(i31, { x: -d16.i * l18 });
      let t30 = Math.max(0, (d16.i - 1) / (u17 - 1));
      o22.textContent = d16.i < 0.6 ? `0` : d16.i >= u17 - 0.02 ? `3,700+` : a20(Math.max(1, 3700 * t30 * t30)), s22.textContent = d16.i < 0.6 ? `Age 10` : d16.i > u17 - 0.6 ? `Today` : `15 years`;
    }, p15 = oi.timeline({ repeat: -1, onUpdate: f15 });
    return p15.set(d16, { i: 0 }), p15.to(d16, { i: u17, duration: 5, ease: `power3.inOut` }, `+=1.2`), p15.to(i31, { opacity: 0, duration: 0.5, ease: `power2.inOut` }, `+=1.6`), p15.set(i31, { opacity: 1 }), f15(), p15;
  } };
  var s16 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var c13 = /* @__PURE__ */ new Map();
  var l15 = /* @__PURE__ */ new Set();
  function u15(e35) {
    let t30 = c13.get(e35);
    if (!t30 || s16) return;
    let n34 = e35.closest(`[data-da-paused], .is-paused`);
    l15.has(e35) && !n34 ? t30.play() : t30.pause();
  }
  function d14(e35) {
    let t30 = c13.get(e35);
    t30 && t30.progress(0).kill();
    let n34 = o15[e35.dataset.da];
    if (!n34 || !e35.clientWidth) return;
    let r31 = n34(e35);
    s16 ? r31.progress(0.55).pause() : r31.pause(), c13.set(e35, r31), u15(e35);
  }
  function f13(e35 = document) {
    let t30 = [...e35.querySelectorAll(`[data-da]:not([data-da-ready])`)];
    if (!t30.length) return;
    t30.forEach((e36) => e36.dataset.daReady = ``);
    let n34 = new IntersectionObserver((e36) => e36.forEach((e37) => {
      let t31 = e37.target;
      e37.isIntersecting ? l15.add(t31) : l15.delete(t31), !c13.has(t31) && e37.isIntersecting ? d14(t31) : u15(t31);
    }), { threshold: 0 });
    t30.forEach((e36) => n34.observe(e36));
    let r31 = 0, i31 = innerWidth;
    addEventListener(`resize`, () => {
      innerWidth !== i31 && (i31 = innerWidth, clearTimeout(r31), r31 = window.setTimeout(() => c13.forEach((e36, t31) => d14(t31)), 250));
    }), new MutationObserver(() => t30.forEach(u15)).observe(e35, { subtree: true, attributes: true, attributeFilter: [`data-da-paused`, `class`] }), e35 === document && !window.__daAgain && (window.__daAgain = true, addEventListener(`load`, () => f13()));
  }

  // HomeDetails.astro_astro_type_script_index_0_lang.CmGwI_bs.js
  var t22 = 6e3;
  function n26(e35) {
    let n34 = e35.querySelector(`[data-slider]`);
    if (!n34) return;
    let r31 = matchMedia(`(prefers-reduced-motion: reduce)`).matches, i31 = false, a29 = false, o22 = 0, s22 = () => {
      Array.from(n34.children).forEach((e36) => {
        let t30 = e36.querySelector(`video[data-fc-video]`);
        t30 && (a29 && !r31 ? (!t30.getAttribute(`src`) && t30.dataset.src && (t30.src = t30.dataset.src), t30.play().catch(() => {
        })) : t30.paused || t30.pause());
      }), e35.classList.toggle(`is-paused`, r31), e35.querySelectorAll(`[data-hd-pause]`).forEach((e36) => {
        e36.setAttribute(`aria-pressed`, String(r31)), e36.setAttribute(`aria-label`, r31 ? `Lecture` : `Pause`);
      });
    }, c17 = () => {
      if (clearTimeout(o22), r31 || i31 || !a29) return;
      let e36 = n34.querySelector(`:scope > .is-active [data-hd-duration]`);
      o22 = window.setTimeout(() => {
        n34.dispatchEvent(new Event(`slider:next`)), c17();
      }, Number(e36?.dataset.hdDuration) || t22);
    };
    new MutationObserver(() => {
      s22(), c17();
    }).observe(n34, { subtree: true, attributes: true, attributeFilter: [`class`] });
    let l18 = e35.querySelector(`[data-hd-gallery]`) ?? e35;
    new IntersectionObserver(([e36]) => {
      a29 = e36.isIntersecting, s22(), c17();
    }, { threshold: 0.4 }).observe(l18), l18.addEventListener(`pointerenter`, (e36) => {
      e36.pointerType === `mouse` && (i31 = true, c17());
    }), l18.addEventListener(`pointerleave`, (e36) => {
      e36.pointerType === `mouse` && (i31 = false, c17());
    }), e35.addEventListener(`focusin`, () => {
      i31 = true, c17();
    }), e35.addEventListener(`focusout`, (t30) => {
      e35.contains(t30.relatedTarget) || (i31 = l18.matches(`:hover`), c17());
    }), e35.addEventListener(`click`, (e36) => {
      e36.target.closest(`[data-hd-pause]`) && (e36.preventDefault(), e36.stopPropagation(), r31 = !r31, s22(), c17());
    }, true), s22();
  }
  var r23 = () => requestAnimationFrame(() => document.querySelectorAll(`[data-home-details]`).forEach(n26));
  document.readyState === `loading` ? document.addEventListener(`DOMContentLoaded`, r23) : r23(), f13();

  // words-marquee.BNOHUsUN.js
  var t23 = matchMedia(`(prefers-reduced-motion: reduce)`).matches;
  var n27 = 36;
  function r24(r31) {
    let i31 = r31.querySelector(`[data-wm-track]`);
    if (!i31) return;
    let a29 = i31.children, o22 = () => a29.length > 1 ? a29[a29.length / 2 | 0].offsetLeft - a29[0].offsetLeft : i31.scrollWidth / 2, s22 = o22();
    addEventListener(`resize`, () => {
      s22 = o22();
    });
    let c17 = 0, l18 = -36, u17 = -1, d16 = false, f15 = 0, p15 = 0, m14 = 0, h12 = false, g14 = 0, _13 = null, v13 = 0;
    r31.addEventListener(`click`, (e35) => {
      e35.target.closest?.(`a[href]`) && e35.detail !== 0 && e35.preventDefault();
    }), r31.addEventListener(`pointerdown`, (e35) => {
      g14 = e35.clientX, v13 = 0, _13 = e35.target.closest?.(`a[href]`) ?? null, d16 = true, f15 = e35.clientX, p15 = performance.now(), m14 = 0, r31.setPointerCapture(e35.pointerId), r31.classList.add(`is-dragging`);
    }), r31.addEventListener(`pointermove`, (e35) => {
      if (!d16) return;
      let t30 = performance.now(), n34 = Math.max(1, t30 - p15), r32 = e35.clientX - f15;
      c17 += r32, v13 = Math.max(v13, Math.abs(e35.clientX - g14)), m14 = m14 * 0.6 + r32 / n34 * 1e3 * 0.4, f15 = e35.clientX, p15 = t30;
    });
    let y13 = (e35) => {
      d16 && (e35.type === `pointerup` && _13 && v13 < 6 && open(_13.href, `_blank`, `noopener`), _13 = null, d16 = false, r31.classList.remove(`is-dragging`), Math.abs(m14) > 20 && (u17 = Math.sign(m14)), l18 = Math.max(-4e3, Math.min(4e3, m14)));
    };
    r31.addEventListener(`pointerup`, y13), r31.addEventListener(`pointercancel`, y13), new IntersectionObserver(([e35]) => {
      h12 = e35.isIntersecting;
    }).observe(r31), oi.ticker.add((e35, r32) => {
      if (!h12) return;
      let a30 = Math.min(0.05, r32 / 1e3);
      if (!d16) {
        let e36 = t23 ? 0 : u17 * n27;
        l18 += (e36 - l18) * Math.min(1, a30 * 1.6), c17 += l18 * a30;
      }
      s22 > 0 && (c17 = (c17 % s22 - s22) % s22), i31.style.transform = `translate3d(${c17.toFixed(2)}px,0,0)`;
    });
  }
  document.querySelectorAll(`[data-wm]`).forEach(r24);

  // parallax.BTz8uNkF.js
  oi.registerPlugin($4);
  var n29 = (e35, t30) => e35 === void 0 || e35 === `` || isNaN(+e35) ? t30 : +e35;
  function r26(t30 = document) {
    let r31 = oi.matchMedia();
    return r31.add({ phone: `(max-width: 767px)`, motion: `(prefers-reduced-motion: no-preference)` }, (r32) => {
      let { phone: i31, motion: a29 } = r32.conditions;
      a29 && (t30.querySelectorAll(`[data-parallax]`).forEach((t31) => {
        let r33 = t31.dataset.parallaxPhone ?? `half`;
        if (i31 && r33 === `off`) return;
        let a30 = i31 && r33 === `half` ? 0.5 : 1, o22 = n29(t31.dataset.parallax, 0.15) * a30, s22 = t31.dataset.parallaxAxis === `x` ? `x` : `y`, c17 = n29(t31.dataset.parallaxSmooth, 0.4), l18 = () => window.innerHeight * o22 * 0.5, u17 = { [s22]: () => l18() }, d16 = { [s22]: () => -l18(), ease: `none` }, f15 = t31.dataset.parallaxRotate;
        f15 && (u17.rotate = -n29(f15, 0) * a30 / 2, d16.rotate = n29(f15, 0) * a30 / 2);
        let p15 = t31.dataset.parallaxScale?.trim().split(/\s+/).map(Number);
        p15 && p15.length === 2 && p15.every((e35) => !isNaN(e35)) && (u17.scale = p15[0], d16.scale = p15[1]), oi.fromTo(t31, u17, { ...d16, scrollTrigger: { trigger: t31, start: `top bottom`, end: `bottom top`, scrub: c17 || true, invalidateOnRefresh: true } });
        let m14 = t31.dataset.parallaxFade;
        m14 && oi.fromTo(t31, { opacity: n29(m14, 0) }, { opacity: 1, ease: `none`, scrollTrigger: { trigger: t31, start: `top bottom`, end: `top 55%`, scrub: c17 || true } });
      }), t30.querySelectorAll(`[data-parallax-media]`).forEach((t31) => {
        let r33 = t31.firstElementChild;
        if (!r33) return;
        let a30 = Math.abs(n29(t31.dataset.parallaxMedia, 0.15)) * (i31 ? 0.6 : 1);
        oi.set(r33, { scale: 1 + a30 * 2, transformOrigin: `50% 50%` }), oi.fromTo(r33, { yPercent: -a30 * 50 }, { yPercent: a30 * 50, ease: `none`, scrollTrigger: { trigger: t31, start: `top bottom`, end: `bottom top`, scrub: n29(t31.dataset.parallaxSmooth, 0.4) || true } });
      }));
    }), r31;
  }

  document.querySelectorAll(`[data-slideshow="wrap"]`).forEach(Ue3), (window.__fontsReady ?? document.fonts.ready).then(We3);
})();
