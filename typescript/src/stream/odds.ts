import type { TxlineClient } from "../client.js";
import type { OddsPayload } from "../http/models.js";
import { typedStream, type SseEvent, type StreamOptions } from "./sse.js";

export class OddsStreamClient {
  constructor(private readonly client: TxlineClient) {}

  stream(options: StreamOptions = {}): AsyncGenerator<SseEvent<OddsPayload>> {
    return typedStream<OddsPayload>(this.client, "/odds/stream", options);
  }
}
