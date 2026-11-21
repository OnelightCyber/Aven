import assert from "node:assert/strict";
import { test } from "node:test";
import {
  BOX_IDLE_MS,
  BOX_WRITES_PER_MINUTE,
  MAX_BLOB_BYTES,
  MAX_MESSAGE_BYTES,
  MAX_PEERS,
  PEER_BURST,
  SUBPROTOCOL,
  admit,
  bearer,
  digest,
  precondition,
  readBounded,
  route,
  same,
  socketToken,
  take,
} from "../src/protocol.js";
import { CLOSE, Room, announce, enter, relay } from "../src/room.js";
import { handleBox } from "../src/sync-box.js";
import worker from "../src/worker.js";

const ID = "a".repeat(64);
const TOKEN = "b".repeat(64);
const OTHER = "c".repeat(64);

class Storage {
  map = new Map();
  alarm = null;
  async get(key) {
    return this.map.get(key);
  }
  async put(key, value) {
    if (typeof key === "string") this.map.set(key, value);
    else for (const [name, entry] of Object.entries(key)) this.map.set(name, entry);
  }
  async deleteAll() {
    this.map.clear();
  }
  async setAlarm(at) {
    this.alarm = at;
  }
  async deleteAlarm() {
    this.alarm = null;
  }
}

const box = (method, { token = TOKEN, body, headers = {} } = {}) =>
  new Request(`https://relay.test/v1/sync/${ID}`, {
    method,
    body,
    headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...headers },
  });

test("only the two versioned paths with a 256-bit id are routed", () => {
  assert.deepEqual(route(`/v1/sync/${ID}`), { kind: "sync", id: ID });
  assert.deepEqual(route(`/v1/room/${ID}`), { kind: "room", id: ID });
  for (const path of ["/", "/v1/sync/", `/v1/sync/${ID}/x`, `/v1/sync/${ID.toUpperCase()}`, `/v2/sync/${ID}`, `/v1/other/${ID}`, `/v1/sync/${"a".repeat(63)}`])
    assert.equal(route(path), null, path);
});

test("a token is read from its header or subprotocol only in the exact form", () => {
  assert.equal(bearer(`Bearer ${TOKEN}`), TOKEN);
  for (const header of [null, "", TOKEN, `bearer ${TOKEN}`, `Bearer ${TOKEN} `, `Bearer ${"b".repeat(63)}`, `Bearer ${"B".repeat(64)}`])
    assert.equal(bearer(header), null);
  assert.equal(socketToken(`${SUBPROTOCOL}, auth.${TOKEN}`), TOKEN);
  assert.equal(socketToken(`auth.${TOKEN},${SUBPROTOCOL}`), TOKEN);
  for (const header of [null, `auth.${TOKEN}`, SUBPROTOCOL, `${SUBPROTOCOL}, auth.short`, `other, auth.${TOKEN}`])
    assert.equal(socketToken(header), null);
});

test("the first token claims an id and only that token passes afterwards", async () => {
  const first = await admit(undefined, TOKEN);
  assert.equal(first.allowed, true);
  assert.equal(first.claim, await digest(TOKEN));
  assert.notEqual(first.claim, TOKEN);
  assert.deepEqual(await admit(first.claim, TOKEN), { allowed: true, claim: null });
  assert.deepEqual(await admit(first.claim, OTHER), { allowed: false, claim: null });
  assert.deepEqual(await admit(undefined, "not a token"), { allowed: false, claim: null });
  assert.equal(same("abc", "abc"), true);
  assert.equal(same("abc", "abd"), false);
  assert.equal(same("abc", "abcd"), false);
});

test("a precondition is a quoted version or the create mark", () => {
  assert.equal(precondition('"7"', null), 7);
  assert.equal(precondition(null, "*"), "*");
  for (const header of [null, "7", '"x"', '"-1"', 'W/"7"']) assert.equal(precondition(header, null), null);
});

test("a body is read up to its limit and no further", async () => {
  const sent = (chunks) => {
    let pulled = 0;
    const body = new ReadableStream({
      pull(controller) {
        if (pulled === chunks.length) return controller.close();
        controller.enqueue(new Uint8Array(chunks[pulled++]));
      },
    });
    const request = new Request("https://relay.test/", { method: "PUT", body, duplex: "half" });
    return { request, pulled: () => pulled };
  };
  const small = sent([3, 4]);
  assert.equal((await readBounded(small.request, 8)).byteLength, 7);
  assert.equal((await readBounded(sent([8]).request, 8)).byteLength, 8);
  // Ten chunks declared by nobody: reading stops soon after the limit.
  const large = sent(Array(10).fill(4));
  assert.equal(await readBounded(large.request, 8), null);
  assert.ok(large.pulled() < 10, `read ${large.pulled()} chunks`);
  assert.equal((await readBounded(new Request("https://relay.test/"), 8)).byteLength, 0);
});

test("a bucket gives its burst, then refills at its rate", () => {
  let bucket;
  for (let n = 0; n < 3; n++) {
    const taken = take(bucket, 1000, 3, 1);
    assert.equal(taken.allowed, true);
    bucket = taken.bucket;
  }
  assert.equal(take(bucket, 1000, 3, 1).allowed, false);
  assert.equal(take(bucket, 1500, 3, 1).allowed, false);
  assert.equal(take(bucket, 2000, 3, 1).allowed, true);
});

test("a box is created, read, and written only on top of the version last read", async () => {
  const storage = new Storage();
  const memory = {};
  const now = 1_000_000;
  // Reading an id nobody wrote claims nothing.
  assert.equal((await handleBox(storage, memory, box("GET"), now)).status, 404);
  assert.equal(storage.map.size, 0);

  const created = await handleBox(storage, memory, box("PUT", { body: "one", headers: { "if-none-match": "*" } }), now);
  assert.equal(created.status, 201);
  assert.equal(created.headers.get("etag"), '"1"');
  assert.equal(storage.map.get("auth"), await digest(TOKEN));
  assert.equal(storage.alarm, now + BOX_IDLE_MS);

  const read = await handleBox(storage, memory, box("GET"), now);
  assert.equal(read.status, 200);
  assert.equal(read.headers.get("etag"), '"1"');
  assert.equal(read.headers.get("cache-control"), "no-store");
  assert.equal(await read.text(), "one");

  // A second create, a write with no version, and a write on a stale one.
  assert.equal((await handleBox(storage, memory, box("PUT", { body: "x", headers: { "if-none-match": "*" } }), now)).status, 412);
  assert.equal((await handleBox(storage, memory, box("PUT", { body: "x" }), now)).status, 428);
  const stale = await handleBox(storage, memory, box("PUT", { body: "x", headers: { "if-match": '"9"' } }), now);
  assert.equal(stale.status, 412);
  assert.equal(stale.headers.get("etag"), '"1"');

  const updated = await handleBox(storage, memory, box("PUT", { body: "two", headers: { "if-match": '"1"' } }), now);
  assert.equal(updated.status, 200);
  assert.equal(updated.headers.get("etag"), '"2"');
  assert.equal(await (await handleBox(storage, memory, box("GET"), now)).text(), "two");
});

test("a box turns away a missing or foreign token, other methods and oversized bodies", async () => {
  const storage = new Storage();
  const memory = {};
  await handleBox(storage, memory, box("PUT", { body: "one", headers: { "if-none-match": "*" } }), 0);
  assert.equal((await handleBox(storage, memory, box("GET", { token: null }), 0)).status, 401);
  assert.equal((await handleBox(storage, memory, box("GET", { token: OTHER }), 0)).status, 403);
  assert.equal((await handleBox(storage, memory, box("PUT", { token: OTHER, body: "x", headers: { "if-match": '"1"' } }), 0)).status, 403);
  assert.equal((await handleBox(storage, memory, box("DELETE", { token: OTHER }), 0)).status, 403);
  assert.equal((await handleBox(storage, memory, box("POST", { body: "x" }), 0)).status, 405);
  const preflight = await handleBox(storage, memory, box("OPTIONS", { token: null }), 0);
  assert.equal(preflight.status, 204);
  assert.equal(preflight.headers.get("access-control-allow-origin"), "*");

  const declared = box("PUT", { body: "x", headers: { "if-match": '"1"', "content-length": String(MAX_BLOB_BYTES + 1) } });
  assert.equal((await handleBox(storage, memory, declared, 0)).status, 413);
  const actual = box("PUT", { body: new Uint8Array(MAX_BLOB_BYTES + 1), headers: { "if-match": '"1"' } });
  assert.equal((await handleBox(storage, memory, actual, 0)).status, 413);
  assert.equal((await handleBox(storage, memory, box("PUT", { body: new Uint8Array(0), headers: { "if-match": '"1"' } }), 0)).status, 400);
  assert.equal(await (await handleBox(storage, memory, box("GET"), 0)).text(), "one");
});

test("a box takes a bounded number of writes a minute, and is forgotten on delete", async () => {
  const storage = new Storage();
  const memory = {};
  let version = 0;
  for (let n = 0; n < BOX_WRITES_PER_MINUTE; n++) {
    const headers = version === 0 ? { "if-none-match": "*" } : { "if-match": `"${version}"` };
    const written = await handleBox(storage, memory, box("PUT", { body: "x", headers }), 0);
    assert.ok(written.status === 200 || written.status === 201, `write ${n}: ${written.status}`);
    version += 1;
  }
  const limited = await handleBox(storage, memory, box("PUT", { body: "x", headers: { "if-match": `"${version}"` } }), 0);
  assert.equal(limited.status, 429);
  assert.equal(limited.headers.get("retry-after"), "60");

  assert.equal((await handleBox(storage, memory, box("DELETE"), 0)).status, 204);
  assert.equal(storage.map.size, 0);
  assert.equal(storage.alarm, null);
  assert.equal((await handleBox(storage, memory, box("DELETE"), 0)).status, 404);
  // Once forgotten, the id can be claimed by another token.
  const reclaimed = await handleBox(storage, {}, box("PUT", { token: OTHER, body: "y", headers: { "if-none-match": "*" } }), 0);
  assert.equal(reclaimed.status, 201);
});

const peer = () => ({
  heard: [],
  closed: null,
  attachment: undefined,
  send(payload) {
    this.heard.push(payload);
  },
  close(code) {
    this.closed = code;
  },
  serializeAttachment(value) {
    this.attachment = value;
  },
  deserializeAttachment() {
    return this.attachment;
  },
});

const roomState = () => {
  const state = {
    storage: new Storage(),
    sockets: [],
    getWebSockets: () => state.sockets,
    acceptWebSocket: (socket) => state.sockets.push(socket),
  };
  return state;
};

const knock = (protocols, upgrade = "websocket") =>
  new Request(`https://relay.test/v1/room/${ID}`, {
    headers: { ...(upgrade ? { upgrade } : {}), ...(protocols ? { "sec-websocket-protocol": protocols } : {}) },
  });

test("a room passes binary messages to the others, untouched, and to nobody else", () => {
  const [a, b, c] = [peer(), peer(), peer()];
  const message = new Uint8Array([1, 2, 3]).buffer;
  const result = relay([a, b, c], a, message, undefined, 0);
  assert.equal(result.close, undefined);
  assert.deepEqual(a.heard, []);
  assert.deepEqual(b.heard, [message]);
  assert.deepEqual(c.heard, [message]);
  announce([a, b]);
  assert.equal(a.heard.at(-1), '{"t":"peers","n":2}');
});

test("a room closes a peer that sends text, too much at once, or too fast", () => {
  const [a, b] = [peer(), peer()];
  assert.deepEqual(relay([a, b], a, "hello", undefined, 0), { close: CLOSE.notBinary });
  assert.deepEqual(relay([a, b], a, new ArrayBuffer(0), undefined, 0), { close: CLOSE.tooLarge });
  assert.deepEqual(relay([a, b], a, new ArrayBuffer(MAX_MESSAGE_BYTES + 1), undefined, 0), { close: CLOSE.tooLarge });
  assert.deepEqual(b.heard, []);
  let bucket;
  for (let n = 0; n < PEER_BURST; n++) bucket = relay([a, b], a, new ArrayBuffer(1), bucket, 0).bucket;
  assert.equal(b.heard.length, PEER_BURST);
  assert.deepEqual(relay([a, b], a, new ArrayBuffer(1), bucket, 0), { close: CLOSE.tooFast });
  assert.equal(relay([a, b], a, new ArrayBuffer(1), bucket, 1000).close, undefined);
});

test("a room admits the claiming token, and refuses a plain request, no token, a foreign one, or a full room", async () => {
  const state = roomState();
  assert.equal((await enter(state, knock(`${SUBPROTOCOL}, auth.${TOKEN}`, null))).status, 426);
  assert.equal((await enter(state, knock(SUBPROTOCOL))).status, 401);
  assert.equal(state.storage.map.size, 0);
  assert.equal(await enter(state, knock(`${SUBPROTOCOL}, auth.${TOKEN}`)), null);
  assert.equal(state.storage.map.get("auth"), await digest(TOKEN));
  assert.equal((await enter(state, knock(`${SUBPROTOCOL}, auth.${OTHER}`))).status, 403);
  for (let n = 0; n < MAX_PEERS; n++) state.sockets.push(peer());
  assert.equal((await enter(state, knock(`${SUBPROTOCOL}, auth.${TOKEN}`))).status, 503);
});

test("a room closes a rule breaker, tells the others who is left, and forgets itself once empty", async () => {
  const state = roomState();
  const room = new Room(state);
  const [a, b] = [peer(), peer()];
  state.sockets.push(a, b);
  await state.storage.put("auth", "digest");
  room.webSocketMessage(a, new Uint8Array([9]).buffer);
  assert.equal(b.heard.length, 1);
  assert.notEqual(a.attachment, undefined);
  room.webSocketMessage(a, "text");
  assert.equal(a.closed, CLOSE.notBinary);

  state.sockets = [b];
  await room.webSocketClose(a);
  assert.equal(b.heard.at(-1), '{"t":"peers","n":1}');
  assert.equal(state.storage.alarm, null);
  state.sockets = [];
  await room.webSocketClose(b);
  assert.notEqual(state.storage.alarm, null);
  await room.alarm();
  assert.equal(state.storage.map.size, 0);
});

test("the worker answers health, refuses unknown paths, and sends an id to its own object", async () => {
  const seen = [];
  const namespace = (kind) => ({
    idFromName: (name) => `${kind}:${name}`,
    get: (id) => ({ fetch: () => (seen.push(id), new Response("routed")) }),
  });
  const env = { SYNC: namespace("sync"), ROOM: namespace("room") };
  assert.equal(await (await worker.fetch(new Request("https://relay.test/v1/health"), env)).text(), "ok");
  assert.equal((await worker.fetch(new Request("https://relay.test/"), env)).status, 404);
  await worker.fetch(new Request(`https://relay.test/v1/sync/${ID}`), env);
  await worker.fetch(new Request(`https://relay.test/v1/room/${ID}`), env);
  assert.deepEqual(seen, [`sync:${ID}`, `room:${ID}`]);
});
