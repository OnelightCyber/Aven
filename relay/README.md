# Aven relay

A small Cloudflare Worker behind two Aven features: sync between one person's
devices, and working together on a canvas. It is a relay and a locker, not a
service that knows anything: every byte it holds or passes on was encrypted
on a device, with a key that never reaches it.

It has no accounts, no dependencies, and nothing to configure.

## What the relay can and cannot see

A client starts from a secret: a sync code shown on one device and typed on
another, or a room link sent to a collaborator. From it the client derives
three values, each with its own label so none gives away another:

- an **id**, 256 bits: where the data lives (`/v1/sync/<id>`, `/v1/room/<id>`);
- a **token**, 256 bits: proof of knowing the secret, sent on each request;
- a **key**: kept on the device and never sent.

The relay stores a SHA-256 digest of the token the first time an id is used,
and asks for the same token afterwards. It sees ids, sizes, timings and IP
addresses. It cannot read, forge or meaningfully alter content: clients
authenticate what they decrypt.

Request logging is off in `wrangler.toml`, because paths carry ids.

## Protocol

All ids and tokens are 64 lowercase hexadecimal characters.

### Sync box

`Authorization: Bearer <token>` on every request.

| Request | Meaning | Answers |
| --- | --- | --- |
| `GET /v1/sync/<id>` | Read the blob | `200` with `ETag: "<version>"`, `404` |
| `PUT` with `If-None-Match: *` | Create | `201`, `412` if it exists |
| `PUT` with `If-Match: "<version>"` | Replace that version | `200` with the new `ETag`, `412` with the current one |
| `DELETE` | Forget the box | `204`, `404` |

A write names the version it was made on top of, so two devices writing at
once cannot silently overwrite each other: the second gets `412`, reads again
and merges. Other answers: `401` no token, `403` wrong token, `413` over
256 KiB, `428` no precondition, `429` over 30 writes a minute.

A box nobody reads or writes for 180 days is deleted.

### Room

`GET /v1/room/<id>` as a WebSocket, with the subprotocols
`aven-relay-v1, auth.<token>`; a browser cannot set headers on a WebSocket, so
the token rides there. The relay answers with `aven-relay-v1`.

- A **binary** message is passed to every other peer as it arrived, and kept
  nowhere. Up to 64 KiB each, 120 at once, then 40 a second.
- A **text** message comes only from the relay: `{"t":"peers","n":3}` whenever
  someone joins or leaves.
- A room holds 8 peers. A peer that sends text, an oversized message or too
  many messages is closed (`1003`, `1009`, `4008`).
- An empty room forgets its token after a day.

## Run the tests

```sh
cd relay
node --test
```

The rules live in plain modules that need no Worker to run; the tests cover
them with a stand-in for storage and sockets, refusals included.

## Deploy

With a Cloudflare account and [Wrangler](https://developers.cloudflare.com/workers/wrangler/):

```sh
cd relay
npx wrangler login
npx wrangler deploy
```

The Worker answers on `https://aven-relay.<your-subdomain>.workers.dev`. No
domain is needed. Durable Objects with SQLite storage are part of the free
plan.

## Known limits

- Anyone can create a box or a room at a new id. Each is bounded in size and
  expires, but the relay itself does not limit how many one address creates:
  add a [rate limiting rule](https://developers.cloudflare.com/waf/rate-limiting-rules/)
  on `/v1/` in the Cloudflare dashboard.
- The Durable Object classes are thin wrappers over the tested rules. They
  are checked by deploying: a box's whole life, a room with two peers and
  the refusals were run against the deployed Worker on 21 November 2026.
