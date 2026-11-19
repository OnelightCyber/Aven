document.documentElement.dataset.avenExtension = chrome.runtime.id;
chrome.runtime.sendMessage({op: "ping"}).then(result => {
  document.documentElement.dataset.avenWorker = JSON.stringify(result);
}).catch(error => {
  document.documentElement.dataset.avenWorkerError = String(error);
});
