// One sync box: the encrypted settings of one person's devices.
//
// The box holds a single blob the relay cannot read, with a version. A
// device writes only on top of the version it last read, so two devices
// writing at once cannot silently overwrite each other: the second is told
// to read again and merge.

import {
  BOX_IDLE_MS,
  BOX_WRITES_PER_MINUTE,
  MAX_BLOB_BYTES,
  admit,
  bearer,
  precondition,
  reply,
  take,
} from "./protocol.js";

const etag = (version) => ({ etag: `"${version}"` });

/** Handles one request against a box's storage. `storage` is the Durable
 *  Object's key-value storage or anything with the same four methods, and
 *  `memory` holds what need not outlive the object: its write budget. */
export async function handleBox(storage, memory, request, now) {
  if (request.method === "OPTIONS") return reply(204);
  if (!["GET", "PUT", "DELETE"].includes(request.method)) return reply(405);
  const token = bearer(request.headers.get("authorization"));
  if (token === null) return reply(401);
  const stored = await storage.get("auth");
  const { allowed, claim } = await admit(stored, token);
  if (!allowed) return reply(403);

  if (request.method === "GET") {
    const version = await storage.get("version");
    if (version === undefined) return reply(404);
    await storage.setAlarm(now + BOX_IDLE_MS);
    return reply(200, await storage.get("blob"), {
      ...etag(version),
      "content-type": "application/octet-stream",
    });
  }

  if (request.method === "DELETE") {
    if (stored === undefined) return reply(404);
    await storage.deleteAll();
    await storage.deleteAlarm();
    return reply(204);
  }

  const budget = take(memory.writes, now, BOX_WRITES_PER_MINUTE, BOX_WRITES_PER_MINUTE / 60);
  memory.writes = budget.bucket;
  if (!budget.allowed) return reply(429, null, { "retry-after": "60" });
  const expected = precondition(
    request.headers.get("if-match"),
    request.headers.get("if-none-match"),
  );
  if (expected === null) return reply(428);
  // Refused by its declared length before a byte is read, then by what was
  // actually sent.
  const declared = Number(request.headers.get("content-length") ?? "0");
  if (!Number.isFinite(declared) || declared > MAX_BLOB_BYTES) return reply(413);
  const blob = await request.arrayBuffer();
  if (blob.byteLength === 0) return reply(400);
  if (blob.byteLength > MAX_BLOB_BYTES) return reply(413);
  const version = await storage.get("version");
  const matches = expected === "*" ? version === undefined : version === expected;
  if (!matches) return reply(412, null, version === undefined ? {} : etag(version));
  const next = (version ?? 0) + 1;
  const record = { version: next, blob };
  if (claim !== null) record.auth = claim;
  await storage.put(record);
  await storage.setAlarm(now + BOX_IDLE_MS);
  return reply(version === undefined ? 201 : 200, null, etag(next));
}

export class SyncBox {
  constructor(state) {
    this.storage = state.storage;
    this.memory = {};
  }

  fetch(request) {
    return handleBox(this.storage, this.memory, request, Date.now());
  }

  /** Nobody came for six months: the box is forgotten. */
  async alarm() {
    await this.storage.deleteAll();
  }
}
