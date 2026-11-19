// Lab-only feasibility check. Records calls, without replacing action behavior.
globalThis.avenLabActionReports = [];
for (const method of ['setTitle', 'setBadgeText', 'setIcon', 'setPopup']) {
  const original = chrome.action[method];
  chrome.action[method] = function (...args) {
    const result = Reflect.apply(original, this, args);
    const details = args[0] || {};
    const image = details.imageData;
    const report = {
      method,
      tabId: details.tabId,
      title: details.title,
      text: details.text,
      popup: details.popup,
      path: details.path,
      image: image && {width: image.width, height: image.height, bytes: image.data?.length}
    };
    globalThis.avenLabActionReports.push(report);
    if (globalThis.avenLabActionReports.length > 32) globalThis.avenLabActionReports.shift();
    return result;
  };
}
