// Pick the nav colour before the first paint (no dark-to-light flash after a page change)
      (function () {
        var bar = document.querySelector("[data-nav-bar]");
        if (!bar) return;
        var root = document.documentElement;
        root.classList.add("nav-instant");
        var r = bar.getBoundingClientRect();
        var y = r.top + r.height / 2;
        var over = Array.prototype.some.call(document.querySelectorAll('[data-nav="inverted"]'), function (s) {
          var b = s.getBoundingClientRect();
          return b.top <= y && b.bottom > y;
        });
        // Coming from another page: start in that page's nav colour; main.ts moves it to this page's colour
        // (with the usual transition) as the page appears, so the nav changes smoothly instead of jumping
        var prev = null;
        try { prev = sessionStorage.getItem("navLight"); sessionStorage.removeItem("navLight"); } catch (e) {}
        bar.classList.toggle("is-light", prev === null ? over : prev === "1");
        requestAnimationFrame(function () { requestAnimationFrame(function () { root.classList.remove("nav-instant"); }); });
      })();
