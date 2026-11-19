// Implements nano-setTimeout-booster (alias nano-stb) from uBlock Origin's wiki, Resources Library: "nano-setTimeout-booster.js".
function avenSetTimeoutBooster(needle, delay, ratio) {
    avenBoostTimer('setTimeout', needle, delay, ratio);
}
