import { describe, expect, it } from "vitest";
import {
  ApiToken,
  GuestJwt,
  TxlineClient,
  activationPreimage,
  buildApiUrl,
  devnetConfig,
  withRpcUrl,
} from "../src/index.js";

describe("auth wrappers", () => {
  it("reject empty tokens and redact formatted output", () => {
    expect(() => new GuestJwt(" ")).toThrow(/must not be empty/u);
    expect(() => new ApiToken("")).toThrow(/must not be empty/u);

    const jwt = new GuestJwt("secret.jwt");
    const token = new ApiToken("secret-api");

    expect(String(jwt)).toBe("GuestJwt(<redacted>)");
    expect(JSON.stringify({ jwt, token })).not.toContain("secret");
  });

  it("builds the activation preimage exactly", () => {
    const jwt = new GuestJwt("jwt");

    expect(activationPreimage("txsig", [1, 2], jwt)).toBe("txsig:1,2:jwt");
    expect(activationPreimage("txsig", [], jwt)).toBe("txsig::jwt");
  });
});

describe("config", () => {
  it("is Devnet-first and rejects obvious mainnet RPC URLs", () => {
    const config = devnetConfig();
    expect(config.network).toBe("devnet");
    expect(config.programId).toBe("6pW64gN1s2uqjHkn1unFeEjAwJkPGHoppGvS715wyP2J");

    expect(() =>
      withRpcUrl(config, "https://api.mainnet-beta.solana.com"),
    ).toThrow(/Devnet RPC/u);
  });
});

describe("REST URL construction", () => {
  it("preserves query parameter names and values", () => {
    const url = buildApiUrl("https://example.test/api", "/scores/stat-validation", [
      ["fixtureId", 10],
      ["seq", 2],
      ["statKeys", "1001,1002"],
      ["ignored", undefined],
    ]);

    expect(url.toString()).toBe(
      "https://example.test/api/scores/stat-validation?fixtureId=10&seq=2&statKeys=1001%2C1002",
    );
  });

  it("uses OpenAPI V2 statKeys instead of repeated legacy statKey params", async () => {
    let requestedUrl = "";
    const fetchImpl = async (input: URL | RequestInfo): Promise<Response> => {
      requestedUrl = String(input);
      return Response.json({
        ts: 1,
        statsToProve: [
          { key: 1001, value: 7, period: 0 },
          { key: 1002, value: 3, period: 0 },
        ],
        eventStatRoot: bytes(10),
        summary: {
          fixtureId: 99,
          updateStats: {
            updateCount: 1,
            minTimestamp: 86_400_000,
            maxTimestamp: 86_400_001,
          },
          eventStatsSubTreeRoot: bytes(11),
        },
        statProofs: [[], []],
        subTreeProof: [],
        mainTreeProof: [],
      });
    };
    const client = new TxlineClient({
      config: devnetConfig(),
      fetch: fetchImpl,
    });
    client.setGuestJwt("guest");
    client.setApiToken("api");

    const response = await client
      .scores()
      .statValidationV2({ fixtureId: 99, seq: 2, statKeys: [1001, 1002] });

    expect(response.requestedStatKeys()).toEqual([1001, 1002]);
    expect(requestedUrl).toContain("statKeys=1001%2C1002");
    expect(requestedUrl).not.toContain("statKey=");
  });
});

function bytes(base: number): number[] {
  return Array.from({ length: 32 }, (_value, index) => (base + index) & 0xff);
}
