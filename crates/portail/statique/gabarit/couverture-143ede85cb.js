// Page transition, step 1: the page starts under the cover (see .pt in base.css); it lifts once the page is ready
      // Scripts wait on window.__pageShown to start their entrance animations as the page appears, not under the cover
      window.__pageShown = new Promise(function (r) { window.__pageShowResolve = r; });
      setTimeout(function () { document.documentElement.classList.add("rv-ready"); }, 4000); // entrances: never keep the first screen hidden
      // Inside a frame (the live setup on /astro): no cover, no intro, no banners (base.css .is-embed)
      var embed = window.self !== window.top;
      if (embed) {
        document.documentElement.classList.add("is-embed");
        // The wheel over the frame scrolls the page around it, unless something in here can still scroll that way
        addEventListener("wheel", function (e) {
          for (var el = e.target; el && el.nodeType === 1; el = el.parentElement) {
            var o = getComputedStyle(el).overflowY;
            if ((o === "auto" || o === "scroll") && el.scrollHeight > el.clientHeight + 1) {
              if (e.deltaY < 0 ? el.scrollTop > 0 : el.scrollTop + el.clientHeight < el.scrollHeight - 1) return;
            }
          }
          try {
            var P = window.parent, ev = new P.WheelEvent("wheel", { deltaX: e.deltaX, deltaY: e.deltaY, deltaMode: e.deltaMode, bubbles: true, cancelable: true });
            P.dispatchEvent(ev);
            if (!ev.defaultPrevented) P.scrollBy(0, e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY);
          } catch (err) {}
        }, { passive: true });
      }
      if (!embed && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
        document.documentElement.classList.add("pt-cover");
        // Never leave the page covered (3.5 s). Here in <head>, not in the script at the end of <body>: a page whose HTML
        // stalls halfway (Safari can hang a connection) never reaches that script and stayed white
        setTimeout(function () { document.documentElement.classList.remove("pt-cover"); window.__pageShowResolve(); }, 3500);
        // The logo intro: once per browser, then again only after 7 days (localStorage "introSeenAt", shared by every
        // tab, link and reload on this domain). Page changes through the transition never show it. ?intro forces it.
        try {
          var seen = +localStorage.getItem("introSeenAt") || 0, now = Date.now();
          if (/[?&]intro\b/.test(location.search) || !(now - seen < 7 * 864e5)) {
            document.documentElement.classList.add("pt-intro");
            localStorage.setItem("introSeenAt", String(now));
          }
        } catch (e) {} // storage blocked: skip the intro rather than show it on every page
      }
      else window.__pageShowResolve();
