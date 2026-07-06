import type { TxlineClient } from "../client.js";
import type { Scores } from "../http/models.js";
import { typedStream, type SseEvent, type StreamOptions } from "./sse.js";

export class ScoresStreamClient {
  constructor(private readonly client: TxlineClient) {}

  stream(options: StreamOptions = {}): AsyncGenerator<SseEvent<Scores>> {
    return typedStream<Scores>(this.client, "/scores/stream", options);
  }
}
