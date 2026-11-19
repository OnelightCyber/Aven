// Refuses the consent banners it recognizes, for the person who asked for it.
// Runs in a world of its own: the page cannot see or change it. It only ever
// clicks a refusal the site itself offers; it sends nothing anywhere, and a
// site that offers no refusal is left alone.
(() => {
  "use strict";
  // A site the person set apart keeps its banner for them to answer.
  const except = globalThis.__avenConsentExcept;
  const here = location.hostname.replace(/^www\./, "");
  if (Array.isArray(except) && except.some((site) => here === site || here.endsWith("." + site))) return;
  const key = "__avenConsentRefusalV1";
  if (Object.prototype.hasOwnProperty.call(globalThis, key)) return;
  Object.defineProperty(globalThis, key, { value: true });

  // How long a document is watched for a late banner, and how often.
  const WATCH_MS = 20000;
  const EVERY_MS = 500;
  const MAX_LOOKS = 60;

  // Refusal buttons of the consent platforms most sites use.
  const KNOWN = [
    "#onetrust-reject-all-handler",
    ".ot-pc-refuse-all-handler",
    "#CybotCookiebotDialogBodyButtonDecline",
    "#CybotCookiebotDialogBodyLevelButtonLevelOptinDeclineAll",
    "#didomi-notice-disagree-button",
    ".didomi-continue-without-agreeing",
    "[data-testid='uc-deny-all-button']",
    ".cmplz-deny",
    ".cky-btn-reject",
    ".osano-cm-denyAll",
    ".iubenda-cs-reject-btn",
    ".klaro .cn-decline",
    "[data-tid='banner-decline']",
    "#tarteaucitronAllDenied2",
    "[data-cookiefirst-action='reject']",
    "#ccc-notify-reject",
    "#ccc-reject-settings",
    ".cmpboxbtnno",
    "#axeptio_btn_dismiss",
    ".fc-cta-do-not-consent",
    "#truste-consent-required",
  ].join(",");

  // Elements that hold a banner in an open shadow tree.
  const SHADOW_HOSTS = "#usercentrics-root,#cmpwrapper,#usercentrics-cmp-ui";

  // Where a banner without a known platform is looked for.
  const BANNERS = [
    "[id*='cookie' i]",
    "[class*='cookie' i]",
    "[id*='consent' i]",
    "[class*='consent' i]",
    "[id*='gdpr' i]",
    "[class*='gdpr' i]",
    "[aria-label*='cookie' i]",
  ].join(",");

  // What a refusal says, whole label, lower case.
  const REFUSALS = new Set([
    "reject all",
    "reject all cookies",
    "decline all",
    "deny all",
    "refuse all",
    "reject",
    "decline",
    "only necessary",
    "necessary only",
    "use necessary cookies only",
    "tout refuser",
    "refuser tout",
    "tout rejeter",
    "refuser",
    "continuer sans accepter",
    "alle ablehnen",
    "ablehnen",
    "nur notwendige",
    "rechazar todo",
    "rechazar todas",
    "rechazar",
    "rifiuta tutto",
    "rifiuta",
    "rejeitar tudo",
    "recusar tudo",
    "alles weigeren",
    "weigeren",
    "odrzuć wszystkie",
    "отклонить все",
    "відхилити всі",
    "tümünü reddet",
    "すべて拒否",
    "모두 거부",
    "全部拒绝",
    "全部拒絕",
  ]);

  const shown = (element) => {
    if (!(element instanceof HTMLElement) || element.hasAttribute("disabled")) return false;
    const box = element.getBoundingClientRect();
    if (box.width < 4 || box.height < 4) return false;
    const style = getComputedStyle(element);
    return style.visibility !== "hidden" && style.display !== "none";
  };

  const label = (element) =>
    (element.innerText || element.getAttribute("aria-label") || "")
      .replace(/\s+/g, " ")
      .trim()
      .toLowerCase();

  const known = (root) => {
    for (const element of root.querySelectorAll(KNOWN)) if (shown(element)) return element;
    return null;
  };

  const worded = () => {
    let seen = 0;
    for (const banner of document.querySelectorAll(BANNERS)) {
      if (++seen > 40) break;
      let buttons = 0;
      for (const button of banner.querySelectorAll("button,[role='button'],a")) {
        if (++buttons > 30) break;
        const text = label(button);
        if (text.length <= 40 && REFUSALS.has(text) && shown(button)) return button;
      }
    }
    return null;
  };

  const refusal = () => {
    const direct = known(document);
    if (direct) return direct;
    for (const host of document.querySelectorAll(SHADOW_HOSTS)) {
      const inside = host.shadowRoot && known(host.shadowRoot);
      if (inside) return inside;
    }
    return worded();
  };

  let looks = 0;
  let done = false;
  let timer = 0;
  let observer = null;

  const stop = () => {
    done = true;
    clearTimeout(timer);
    if (observer) observer.disconnect();
  };

  const look = () => {
    timer = 0;
    if (done) return;
    if (++looks > MAX_LOOKS) return stop();
    let button = null;
    try {
      button = refusal();
    } catch {
      return stop();
    }
    if (!button) return;
    stop();
    button.click();
  };

  const soon = () => {
    if (!done && !timer) timer = setTimeout(look, EVERY_MS);
  };

  const start = () => {
    if (!document.documentElement) return;
    observer = new MutationObserver(soon);
    observer.observe(document.documentElement, { childList: true, subtree: true });
    setTimeout(stop, WATCH_MS);
    soon();
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start, { once: true });
  } else {
    start();
  }
})();
