// Plays this tab's sound at the level the person chose for it. A media
// element keeps the volume the page set and reports it unchanged: what is
// heard is that volume times the level. Web Audio goes through one gain per
// context. Best effort: it runs in the page's own world, so a page can undo
// it for itself, and sound in another site's frame keeps its own loudness
// until that frame loads again. It sends nothing anywhere.
(() => {
  "use strict";
  const KEY = "__avenTabVolumeV1";
  const level = Math.min(Math.max(Number(AVEN_TAB_VOLUME) / 100, 0), 1);
  const known = globalThis[KEY];
  if (known && typeof known.set === "function") { known.set(level); return; }

  let current = level;
  const proto = globalThis.HTMLMediaElement && HTMLMediaElement.prototype;
  const own = proto && Object.getOwnPropertyDescriptor(proto, "volume");
  // What each element was asked to play at, by the page.
  const asked = new WeakMap();
  const scale = (element) => {
    if (!own) return;
    if (!asked.has(element)) asked.set(element, own.get.call(element));
    own.set.call(element, Math.min(1, asked.get(element) * current));
  };
  const everyElement = () => {
    try { for (const element of document.querySelectorAll("audio,video")) scale(element); } catch (_) {}
  };

  if (own && own.get && own.set) {
    Object.defineProperty(proto, "volume", {
      configurable: true,
      enumerable: own.enumerable,
      get() { return asked.has(this) ? asked.get(this) : own.get.call(this); },
      set(value) {
        // The browser's own check first: a wrong value throws as it always did.
        own.set.call(this, value);
        asked.set(this, own.get.call(this));
        own.set.call(this, Math.min(1, asked.get(this) * current));
      },
    });
    const play = proto.play;
    proto.play = function () { try { scale(this); } catch (_) {} return play.apply(this, arguments); };
    // Media events do not bubble; they are caught on the way down.
    document.addEventListener("play", (event) => {
      if (event.target instanceof HTMLMediaElement) scale(event.target);
    }, true);
  }

  // Web Audio: whatever a page sends to a context's output goes through
  // one gain, made the first time something is connected there.
  const masters = new WeakMap();
  const gains = [];
  const node = globalThis.AudioNode && AudioNode.prototype;
  if (node && globalThis.AudioDestinationNode) {
    const connect = node.connect, disconnect = node.disconnect;
    const master = (context) => {
      let gain = masters.get(context);
      if (!gain) {
        gain = context.createGain();
        gain.gain.value = current;
        connect.call(gain, context.destination);
        masters.set(context, gain);
        gains.push(new WeakRef(gain));
      }
      return gain;
    };
    node.connect = function (target, ...rest) {
      if (target instanceof AudioDestinationNode) {
        try { connect.call(this, master(target.context), ...rest); return target; } catch (_) {}
      }
      return connect.call(this, target, ...rest);
    };
    node.disconnect = function (target, ...rest) {
      if (target instanceof AudioDestinationNode && masters.has(target.context)) {
        return disconnect.call(this, masters.get(target.context), ...rest);
      }
      return disconnect.apply(this, arguments);
    };
  }

  Object.defineProperty(globalThis, KEY, {
    value: {
      set(next) {
        current = next;
        everyElement();
        for (const held of gains) {
          const gain = held.deref();
          if (gain) gain.gain.value = current;
        }
      },
    },
  });
  everyElement();
})();
