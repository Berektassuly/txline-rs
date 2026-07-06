import { describe, expect, it } from "vitest";
import { SseDecoder, parseSseBlock, typedEventFromRaw } from "../src/index.js";

describe("SSE parsing", () => {
  it("parses id, event, retry, and multi-line data", () => {
    const event = parseSseBlock(
      [
        "id: 42",
        "event: scores",
        "retry: 2500",
        'data: {"a":1,',
        'data: "b":2}',
      ].join("\n"),
    );

    expect(event).toEqual({
      id: "42",
      event: "scores",
      retry: 2500,
      data: '{"a":1,\n"b":2}',
    });
  });

  it("decodes streamed chunks and filters heartbeat events before JSON parsing", () => {
    const decoder = new SseDecoder();
    const events = decoder.push(
      new TextEncoder().encode(
        [
          "id: hb",
          "event: heartbeat",
          "data: not-json",
          "",
          "id: score-1",
          "event: scores",
          'data: {"fixtureId":1}',
          "",
          "",
        ].join("\n"),
      ),
    );

    expect(events).toHaveLength(2);
    expect(typedEventFromRaw(events[0]!)).toBeUndefined();
    expect(typedEventFromRaw<{ fixtureId: number }>(events[1]!)).toEqual({
      id: "score-1",
      event: "scores",
      data: { fixtureId: 1 },
    });
  });
});
