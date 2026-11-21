// One room: a few people working on the same canvas.
//
// The relay passes each message, as the opaque bytes it arrived as, to
// everyone else in the room. It keeps none of them. What the relay itself
// says is text; what people send each other is binary and encrypted with a
// key the relay never had.

import {
  MAX_MESSAGE_BYTES,
  MAX_PEERS,
  PEER_BURST,
  PEER_PER_SECOND,
  ROOM_IDLE_MS,
  SUBPROTOCOL,
  admit,
  reply,
  socketToken,
  take,
} from "./protocol.js";

/** Close codes a client can tell apart. */
export const CLOSE = { tooLarge: 1009, notBinary: 1003, tooFast: 4008 };

const send = (socket, payload) => {
  try {
    socket.send(payload);
  } catch {
    /* A socket closing under us has nothing more to hear. */
  }
};

/** Tells everyone how many are in the room. */
export function announce(sockets) {
  const text = JSON.stringify({ t: "peers", n: sockets.length });
  for (const socket of sockets) send(socket, text);
}

/** One message from one peer: bounded, binary, within its rate, then passed
 *  to the others untouched. Returns the peer's bucket to keep, or a close
 *  code when the peer broke a rule. */
export function relay(sockets, sender, message, bucket, now) {
  if (typeof message === "string") return { close: CLOSE.notBinary };
  if (message.byteLength === 0 || message.byteLength > MAX_MESSAGE_BYTES)
    return { close: CLOSE.tooLarge };
  const taken = take(bucket, now, PEER_BURST, PEER_PER_SECOND);
  if (!taken.allowed) return { close: CLOSE.tooFast };
  for (const socket of sockets) if (socket !== sender) send(socket, message);
  return { bucket: taken.bucket };
}

/** Whether this caller may join: a refusal to send back, or nothing when
 *  the door is open. The first token to reach a room claims it. */
export async function enter(state, request) {
  if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") return reply(426);
  const token = socketToken(request.headers.get("sec-websocket-protocol"));
  if (token === null) return reply(401);
  const { allowed, claim } = await admit(await state.storage.get("auth"), token);
  if (!allowed) return reply(403);
  if (state.getWebSockets().length >= MAX_PEERS) return reply(503);
  if (claim !== null) await state.storage.put("auth", claim);
  await state.storage.deleteAlarm();
  return null;
}

export class Room {
  constructor(state) {
    this.state = state;
  }

  async fetch(request) {
    const refused = await enter(this.state, request);
    if (refused !== null) return refused;
    const pair = new WebSocketPair();
    // Hibernation: the room costs nothing while nobody speaks.
    this.state.acceptWebSocket(pair[1]);
    announce(this.state.getWebSockets());
    return new Response(null, {
      status: 101,
      webSocket: pair[0],
      headers: { "sec-websocket-protocol": SUBPROTOCOL },
    });
  }

  webSocketMessage(socket, message) {
    const result = relay(
      this.state.getWebSockets(),
      socket,
      message,
      socket.deserializeAttachment(),
      Date.now(),
    );
    if (result.close !== undefined) socket.close(result.close);
    else socket.serializeAttachment(result.bucket);
  }

  async webSocketClose(socket) {
    const left = this.state.getWebSockets().filter((other) => other !== socket);
    announce(left);
    if (left.length === 0) await this.state.storage.setAlarm(Date.now() + ROOM_IDLE_MS);
  }

  webSocketError(socket) {
    return this.webSocketClose(socket);
  }

  /** Empty for a day: the room forgets its token and can be claimed anew. */
  async alarm() {
    if (this.state.getWebSockets().length === 0) await this.state.storage.deleteAll();
  }
}
