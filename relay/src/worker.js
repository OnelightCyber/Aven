// Aven relay: encrypted sync between one person's devices, and rooms for
// working together. It routes each id to its own Durable Object and holds
// no state of its own. See ../README.md.

import { reply, route } from "./protocol.js";

export { Room } from "./room.js";
export { SyncBox } from "./sync-box.js";

export default {
  fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/v1/health") return reply(200, "ok", { "content-type": "text/plain" });
    const target = route(url.pathname);
    if (target === null) return reply(404);
    const namespace = target.kind === "sync" ? env.SYNC : env.ROOM;
    return namespace.get(namespace.idFromName(target.id)).fetch(request);
  },
};
