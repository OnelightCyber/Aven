// The relay's wire rules, with nothing in them that needs a Worker to run.
//
// The relay never sees a key or readable content. A client derives three
// values from a secret that only its own devices or its collaborators hold:
// an id (where), a token (who may write there) and a key (kept by the
// client, never sent). The relay stores a digest of the token the first time
// an id is used and asks for the same token ever after.

export const MAX_BLOB_BYTES = 256 * 1024;
export const MAX_MESSAGE_BYTES = 64 * 1024;
export const MAX_PEERS = 8;
/** A box nobody has read or written for this long is deleted. */
export const BOX_IDLE_MS = 180 * 24 * 60 * 60 * 1000;
/** A room with nobody in it for this long forgets its token. */
export const ROOM_IDLE_MS = 24 * 60 * 60 * 1000;
/** Writes one box takes each minute. */
export const BOX_WRITES_PER_MINUTE = 30;
/** Messages one peer may send: a burst, then a steady rate each second. */
export const PEER_BURST = 120;
export const PEER_PER_SECOND = 40;

export const SUBPROTOCOL = "aven-relay-v1";
const AUTH_PREFIX = "auth.";
const HEX_64 = /^[0-9a-f]{64}$/u;

/** An id or a token: 256 bits as lowercase hexadecimal. */
export const hex64 = (value) => typeof value === "string" && HEX_64.test(value);

/** `/v1/sync/<id>` or `/v1/room/<id>`, or nothing for any other path. */
export function route(pathname) {
  const match = /^\/v1\/(sync|room)\/([0-9a-f]{64})$/u.exec(pathname);
  return match ? { kind: match[1], id: match[2] } : null;
}

/** The token of `Authorization: Bearer <token>`, or nothing. */
export function bearer(header) {
  const match = /^Bearer ([0-9a-f]{64})$/u.exec(header ?? "");
  return match ? match[1] : null;
}

/** The token a WebSocket client offers beside the relay's own subprotocol.
 *  A browser cannot set headers on a WebSocket, so it rides there. */
export function socketToken(header) {
  const offered = (header ?? "").split(",").map((part) => part.trim());
  if (!offered.includes(SUBPROTOCOL)) return null;
  const auth = offered.find((part) => part.startsWith(AUTH_PREFIX));
  const token = auth?.slice(AUTH_PREFIX.length);
  return hex64(token) ? token : null;
}

export async function digest(token) {
  const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(token));
  return [...new Uint8Array(bytes)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Compares two digests without stopping at the first difference. */
export function same(left, right) {
  if (typeof left !== "string" || typeof right !== "string" || left.length !== right.length)
    return false;
  let difference = 0;
  for (let index = 0; index < left.length; index++)
    difference |= left.charCodeAt(index) ^ right.charCodeAt(index);
  return difference === 0;
}

/** The first token to reach an id claims it; afterwards only that token
 *  passes. Returns whether this token may act, and the digest to store when
 *  it is the first. */
export async function admit(storedDigest, token) {
  if (!hex64(token)) return { allowed: false, claim: null };
  const offered = await digest(token);
  if (storedDigest === undefined || storedDigest === null)
    return { allowed: true, claim: offered };
  return { allowed: same(storedDigest, offered), claim: null };
}

/** A request's body, or nothing once it holds more than `limit` bytes.
 *  Reading stops there, so a caller that declares no length cannot make the
 *  relay hold a body it is going to refuse. */
export async function readBounded(request, limit) {
  if (!request.body) return new ArrayBuffer(0);
  const reader = request.body.getReader();
  const chunks = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes.buffer;
}

/** The version of `If-Match: "<n>"`, `*` for `If-None-Match: *`, or nothing. */
export function precondition(ifMatch, ifNoneMatch) {
  if (ifNoneMatch === "*") return "*";
  const match = /^"(\d{1,15})"$/u.exec(ifMatch ?? "");
  return match ? Number(match[1]) : null;
}

/** A token bucket: `burst` at once, refilled at `perSecond`. Returns the
 *  bucket after one take, and whether the take was allowed. */
export function take(bucket, now, burst, perSecond) {
  const last = bucket ?? { tokens: burst, at: now };
  const refilled = Math.min(burst, last.tokens + ((now - last.at) / 1000) * perSecond);
  if (refilled < 1) return { allowed: false, bucket: { tokens: refilled, at: now } };
  return { allowed: true, bucket: { tokens: refilled - 1, at: now } };
}

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, PUT, DELETE, OPTIONS",
  "access-control-allow-headers": "authorization, content-type, if-match, if-none-match",
  "access-control-expose-headers": "etag",
  "access-control-max-age": "86400",
};

/** A response that says nothing about the caller and is never cached. */
export function reply(status, body = null, headers = {}) {
  return new Response(body, {
    status,
    headers: { "cache-control": "no-store", "x-content-type-options": "nosniff", ...CORS, ...headers },
  });
}
