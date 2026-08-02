// Custom JavaScript to modify the mdbook menu bar
// Runs after the page loads

document.addEventListener("DOMContentLoaded", function () {
  setTimeout(function () {
    modifyMenuBar();
  }, 100);
});

function modifyMenuBar() {
  const menuBar = document.getElementById("mdbook-menu-bar");
  if (!menuBar) return;

  // 1. Remove the "JereIDE Wiki" title
  const title = menuBar.querySelector(".menu-title");
  if (title) title.remove();

  // 2. Convert mdBook's native alerts to the site's markdown-alert style
  convertAlerts();

  // 3. Add a JereIDE home icon to the left buttons
  const leftButtons = menuBar.querySelector(".left-buttons");
  if (leftButtons && !leftButtons.querySelector(".home-link")) {
    const homeLink = document.createElement("a");
    homeLink.className = "home-link";
    homeLink.href = "../";
    homeLink.title = "JereIDE Home";
    homeLink.setAttribute("aria-label", "JereIDE Home");

    const iconImg = document.createElement("img");
    iconImg.src = "../assets/AppIcon.png";
    iconImg.alt = "JereIDE";
    iconImg.width = 24;
    iconImg.height = 24;

    homeLink.appendChild(iconImg);
    leftButtons.prepend(homeLink);
  }

  // 4. Add the Auto/Dark/Light theme slider
  if (leftButtons && !leftButtons.querySelector(".theme-slider")) {
    const themeToggle = menuBar.querySelector("#mdbook-theme-toggle");
    const slider = document.createElement("div");
    slider.className = "theme-slider";
    slider.setAttribute("role", "group");
    slider.setAttribute("aria-label", "Theme");

    const thumb = document.createElement("div");
    thumb.className = "theme-slider-thumb";
    slider.appendChild(thumb);

    ["auto", "dark", "light"].forEach(function (name) {
      const btn = document.createElement("button");
      btn.className = "theme-slider-opt";
      btn.type = "button";
      btn.dataset.theme = name;
      btn.textContent = name.charAt(0).toUpperCase() + name.slice(1);
      btn.addEventListener("click", function () {
        applyTheme(name);
      });
      slider.appendChild(btn);
    });

    if (themeToggle) {
      leftButtons.insertBefore(slider, themeToggle);
    } else {
      leftButtons.appendChild(slider);
    }
    syncSlider();
  }

  // 5. Move the search results popup to <body> so no ancestor transform/overflow can clip it
  const resultsOuter = document.getElementById("mdbook-searchresults-outer");
  if (resultsOuter) {
    document.body.appendChild(resultsOuter);

    if (!resultsOuter.querySelector(".searchresults-close")) {
      const closeBtn = document.createElement("button");
      closeBtn.className = "searchresults-close";
      closeBtn.type = "button";
      closeBtn.textContent = "\u00d7";
      closeBtn.title = "Close search results";
      closeBtn.setAttribute("aria-label", "Close search results");
      resultsOuter.appendChild(closeBtn);
      closeBtn.addEventListener("click", function () {
        resultsOuter.classList.add("hidden");
      });
    }
  }

  // 6. Preload the search index so searching works immediately (mdbook only
  //    loads it when the search icon is clicked, which we have hidden).
  const searchToggle = document.getElementById("mdbook-search-toggle");
  if (searchToggle) {
    searchToggle.click();
  }

  // Set a custom search placeholder
  const searchbar = document.getElementById("mdbook-searchbar");
  if (searchbar) {
    searchbar.placeholder = "Search docs...";
  }

  // 7. Keep the sidebar visible and prevent toggling it off
  const sidebarToggleAnchor = document.getElementById(
    "mdbook-sidebar-toggle-anchor",
  );
  if (sidebarToggleAnchor) {
    sidebarToggleAnchor.checked = true;
    document.documentElement.classList.add("sidebar-visible");
  }
}

function convertAlerts() {
  document
    .querySelectorAll(".content blockquote.blockquote-tag")
    .forEach(function (bq) {
      let type = "";
      bq.classList.forEach(function (cls) {
        const m = cls.match(/^blockquote-tag-(.+)$/);
        if (m) type = m[1];
      });
      if (!type) return;

      const alert = document.createElement("div");
      alert.className = "markdown-alert markdown-alert-" + type;

      const titleP = bq.querySelector(".blockquote-tag-title");
      let titleText = type;
      if (titleP) {
        titleText = (titleP.textContent || "").trim() || type;
        titleP.remove();
      }
      const title = document.createElement("p");
      title.className = "markdown-alert-title";
      title.textContent = titleText;
      alert.appendChild(title);

      while (bq.firstChild) {
        alert.appendChild(bq.firstChild);
      }
      bq.replaceWith(alert);
    });
}

function getSliderTheme() {
  let saved = null;
  try {
    saved = localStorage.getItem("mdbook-theme");
  } catch (e) {}
  if (!saved || saved === "default_theme") return "auto";
  if (saved === "coal" || saved === "navy" || saved === "ayu") return "dark";
  return "light";
}

function syncSlider() {
  const slider = document.querySelector(".theme-slider");
  if (!slider) return;
  const current = getSliderTheme();
  slider.dataset.theme = current;
  slider.querySelectorAll(".theme-slider-opt").forEach(function (btn) {
    btn.dataset.active = btn.dataset.theme === current ? "true" : "false";
  });
}

function applyTheme(mode) {
  const darkSystem =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved =
    mode === "auto"
      ? darkSystem
        ? "coal"
        : "light"
      : mode === "dark"
        ? "coal"
        : "light";

  ["light", "rust", "coal", "navy", "ayu", "default_theme"].forEach(
    function (t) {
      document.documentElement.classList.remove(t);
    },
  );
  document.documentElement.classList.add(resolved);

  const ayu = document.getElementById("mdbook-ayu-highlight-css");
  const tn = document.getElementById("mdbook-tomorrow-night-css");
  const hl = document.getElementById("mdbook-highlight-css");
  if (resolved === "light") {
    if (ayu) ayu.disabled = true;
    if (tn) tn.disabled = true;
    if (hl) hl.disabled = false;
  } else {
    if (ayu) ayu.disabled = true;
    if (tn) tn.disabled = false;
    if (hl) hl.disabled = true;
  }

  try {
    if (mode === "auto") {
      localStorage.removeItem("mdbook-theme");
    } else {
      localStorage.setItem("mdbook-theme", resolved);
    }
  } catch (e) {}

  syncSlider();
}
