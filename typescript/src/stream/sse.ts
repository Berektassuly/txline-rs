import { InvalidInputError } from "../errors.js";
import type { TxlineClient } from "../client.js";
import type { QueryEntries } from "../http/request.js";

export interface RawSseEvent {
  readonly id?: string;
  readonly event?: string;
  readonly data: string;
  readonly retry?: number;
}

export interface SseEvent<T> {
  readonly id?: string;
  readonly event?: string;
  readonly data: T;
}

export interface StreamOptions {
  readonly fixtureId?: number;
  readonly lastEventId?: string;
  readonly initialBackoffMs?: number;
  readonly maxBackoffMs?: number;
  readonly signal?: AbortSignal;
}

const DEFAULT_INITIAL_BACKOFF_MS = 1_000;
const DEFAULT_MAX_BACKOFF_MS = 30_000;

export class SseDecoder {
  #buffer = "";
  #decoder = new TextDecoder("utf-8", { fatal: true });

  push(bytes: Uint8Array): RawSseEvent[] {
    try {
      this.#buffer += this.#decoder.decode(bytes, { stream: true });
    } catch (error) {
      throw new InvalidInputError(`SSE utf8 error: ${String(error)}`);
    }
    const events: RawSseEvent[] = [];
    for (;;) {
      const split = splitSseBlock(this.#buffer);
      if (!split) {
        break;
      }
      const parsed = parseSseBlock(split.block);
      this.#buffer = split.remainder;
      if (parsed) {
        events.push(parsed);
      }
    }
    return events;
  }

  finish(): RawSseEvent | undefined {
    const tail = this.#decoder.decode();
    if (tail.length > 0) {
      this.#buffer += tail;
    }
    if (this.#buffer.trim().length === 0) {
      this.#buffer = "";
      return undefined;
    }
    const parsed = parseSseBlock(this.#buffer);
    this.#buffer = "";
    return parsed;
  }
}

export function parseSseBlock(block: string): RawSseEvent | undefined {
  let id: string | undefined;
  let event: string | undefined;
  let data = "";
  let retry: number | undefined;

  for (const rawLine of block.split(/\r?\n/u)) {
    if (rawLine.length === 0 || rawLine.startsWith(":")) {
      continue;
    }
    const separator = rawLine.indexOf(":");
    const field = separator === -1 ? rawLine : rawLine.slice(0, separator);
    let value = separator === -1 ? "" : rawLine.slice(separator + 1);
    if (value.startsWith(" ")) {
      value = value.slice(1);
    }
    switch (field) {
      case "id":
        id = value;
        break;
      case "event":
        event = value;
        break;
      case "data":
        data += `${value}\n`;
        break;
      case "retry": {
        const parsed = Number.parseInt(value, 10);
        if (Number.isSafeInteger(parsed) && parsed >= 0) {
          retry = parsed;
        }
        break;
      }
      default:
        break;
    }
  }

  if (data.endsWith("\n")) {
    data = data.slice(0, -1);
  }
  if (!id && !event && data.length === 0 && retry === undefined) {
    return undefined;
  }
  return {
    ...(id !== undefined ? { id } : {}),
    ...(event !== undefined ? { event } : {}),
    data,
    ...(retry !== undefined ? { retry } : {}),
  };
}

export async function* typedStream<T>(
  client: TxlineClient,
  path: string,
  options: StreamOptions = {},
): AsyncGenerator<SseEvent<T>, void, void> {
  let lastEventId = options.lastEventId;
  let backoffMs = options.initialBackoffMs ?? DEFAULT_INITIAL_BACKOFF_MS;
  const initialBackoffMs = backoffMs;
  const maxBackoffMs = options.maxBackoffMs ?? DEFAULT_MAX_BACKOFF_MS;

  while (!options.signal?.aborted) {
    const query: QueryEntries =
      options.fixtureId === undefined ? [] : [["fixtureId", options.fixtureId]];
    try {
      const response = await client.sseResponse(
        path,
        query,
        lastEventId,
        options.signal,
      );
      backoffMs = initialBackoffMs;
      const decoder = new SseDecoder();
      for await (const chunk of responseBytes(response)) {
        for (const raw of decoder.push(chunk)) {
          if (raw.id !== undefined) {
            lastEventId = raw.id;
          }
          if (raw.retry !== undefined) {
            backoffMs = Math.min(raw.retry, maxBackoffMs);
          }
          const event = typedEventFromRaw<T>(raw);
          if (event) {
            yield event;
          }
        }
      }
      const tail = decoder.finish();
      if (tail) {
        if (tail.id !== undefined) {
          lastEventId = tail.id;
        }
        if (tail.retry !== undefined) {
          backoffMs = Math.min(tail.retry, maxBackoffMs);
        }
        const event = typedEventFromRaw<T>(tail);
        if (event) {
          yield event;
        }
      }
    } catch (error) {
      if (options.signal?.aborted) {
        return;
      }
      throw error;
    }

    await sleep(backoffMs, options.signal);
    backoffMs = Math.min(backoffMs * 2, maxBackoffMs);
  }
}

export function typedEventFromRaw<T>(raw: RawSseEvent): SseEvent<T> | undefined {
  if (raw.data.length === 0 || raw.event?.toLowerCase() === "heartbeat") {
    return undefined;
  }
  const data = JSON.parse(raw.data) as T;
  return {
    ...(raw.id !== undefined ? { id: raw.id } : {}),
    ...(raw.event !== undefined ? { event: raw.event } : {}),
    data,
  };
}

function splitSseBlock(buffer: string): { block: string; remainder: string } | undefined {
  const lf = buffer.indexOf("\n\n");
  const crlf = buffer.indexOf("\r\n\r\n");
  let index = -1;
  let separatorLength = 0;
  if (lf !== -1 && (crlf === -1 || lf < crlf)) {
    index = lf;
    separatorLength = 2;
  } else if (crlf !== -1) {
    index = crlf;
    separatorLength = 4;
  }
  if (index === -1) {
    return undefined;
  }
  return {
    block: buffer.slice(0, index),
    remainder: buffer.slice(index + separatorLength),
  };
}

async function* responseBytes(response: Response): AsyncGenerator<Uint8Array> {
  if (!response.body) {
    return;
  }
  const reader = response.body.getReader();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        return;
      }
      yield value;
    }
  } finally {
    reader.releaseLock();
  }
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  if (signal?.aborted) {
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    const timeout = setTimeout(resolve, ms);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timeout);
        resolve();
      },
      { once: true },
    );
  });
}
