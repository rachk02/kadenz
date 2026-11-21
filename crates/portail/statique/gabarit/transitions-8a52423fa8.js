/* Page transitions (a plain cover, no View Transitions: their snapshots were what made pages flash twice).
         Leaving: the cover fades in over the page (with a blur), then the browser goes to the next page, which was
         already prefetched on hover. The browser holds that last frame until the new page paints, and the new page
         paints with the same cover on, so the switch is invisible. Arriving: once fonts and the first images are
         ready the cover fades and unblurs away, and entrance animations start right then. The nav stays above it. */
      (function () {
        var root = document.documentElement;
        var reveal = function () { root.classList.remove("pt-cover"); window.__pageShowResolve(); };

        // The two fonts of the site, asked for by name. Not document.fonts.ready: WebKit (Safari) keeps that one
        // waiting until the whole page has loaded, pictures and all, so every page change sat under the cover for
        // the full 0.7 s and entrances started late. reveal.ts and main.ts wait on this too.
        window.__fontsReady = document.fonts && document.fonts.load
          ? Promise.all([document.fonts.load("1em Diatype"), document.fonts.load('1em "Geist Mono"')]).catch(function () {})
          : Promise.resolve();

        if (root.classList.contains("pt-cover")) {
          var ready = function () {
            var imgs = Array.prototype.slice.call(document.querySelectorAll("img")).filter(function (i) {
              // Images that fade themselves in ([data-fade]) never hold the page back: text and layout come first
              if (i.hasAttribute("data-fade")) return false;
              var r = i.getBoundingClientRect(); return r.top < innerHeight && r.bottom > 0 && r.width > 0;
            }).slice(0, 2);
            var waits = [window.__fontsReady].concat(imgs.map(function (i) { return i.decode ? i.decode().catch(function () {}) : null; }));
            var loaded = Promise.race([Promise.all(waits), new Promise(function (r) { setTimeout(r, 700); })]); // a slow image never holds the page back for long
            if (root.classList.contains("pt-intro")) {
              // Intro: the logo fills (at least once through), slides down out of its frame, then the cover lifts
              Promise.all([loaded, new Promise(function (r) { setTimeout(r, 1250); })]).then(function () {
                root.classList.add("pt-intro-out");
                setTimeout(function () { reveal(); setTimeout(function () { root.classList.remove("pt-intro", "pt-intro-out"); }, 1200); }, 700);
              });
              return;
            }
            loaded.then(function () {
              requestAnimationFrame(function () { requestAnimationFrame(reveal); });
            });
          };
          // This script sits at the end of <body>, so the page is already parsed: start now instead of waiting for
          // DOMContentLoaded (which also waits for every module script and made phones sit on black for a while)
          ready(); // (the 3.5 s "never leave the page covered" timer is in <head>, so it runs even if the HTML stalls)
        }

        // Back/forward from the cache: show the page as it was, without the cover
        addEventListener("pageshow", function (e) { if (e.persisted) { reveal(); root.classList.remove("pt-intro", "pt-intro-out"); var pt = document.querySelector(".pt"); if (pt) { pt.style.display = ""; pt.style.transition = ""; } } });

        var go = function (url) {
          if (root.classList.contains("pt-cover") || matchMedia("(prefers-reduced-motion: reduce)").matches) { location.href = url; return; }
          var pt = document.querySelector(".pt");
          if (pt) { pt.style.transition = ""; pt.style.display = "block"; void pt.offsetWidth; } // from display: none, so the fade-in runs
          root.classList.add("pt-cover");
          try { var bar = document.querySelector("[data-nav-bar]"); if (bar) sessionStorage.setItem("navLight", bar.classList.contains("is-light") ? "1" : "0"); } catch (e) {}
          setTimeout(function () {
            // Pin the cover at full black before leaving: when a navigation starts, WebKit drops a running (or just
            // finished) transition back to its start, so the old page flashed uncovered until the next one painted
            if (pt) pt.style.transition = "none";
            location.href = url;
            // The next page never came (Safari can hang a connection): after 3 s show this page again.
            // WebKit freezes CSS transitions while a navigation is pending, so taking pt-cover off alone isn't enough:
            // the cover is hidden at once, without a transition (pageshow and the next go() put both back)
            setTimeout(function () {
              var c = document.querySelector(".pt");
              if (c) { c.style.transition = "none"; c.style.display = "none"; }
              root.classList.remove("pt-cover");
            }, 3000);
          }, 380);
        };
        window.__pageGo = go;

        document.addEventListener("click", function (e) {
          if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
          var a = e.target.closest && e.target.closest("a[href]");
          if (!a || (a.target && a.target !== "_self") || a.hasAttribute("download") || a.dataset.noTransition !== undefined) return;
          var url = new URL(a.href, location.href);
          if (url.origin !== location.origin || url.pathname.indexOf("/api/") === 0) return; // downloads and sign-ins never cover the page
          if (url.pathname === location.pathname && url.search === location.search) return; // same page / anchor
          e.preventDefault();
          go(url.href);
        });
      })();
