import { InvalidInputError } from "../errors.js";
import type { TxlineClient } from "../client.js";
import { ScoresStatValidationV2 } from "../validation/v2.js";
import type { ScoresStatValidation, ScoresStatValidationV2Response } from "../validation/index.js";
import { validateHour, validateInterval } from "./fixtures.js";
import type { Scores } from "./models.js";

export class ScoresClient {
  constructor(private readonly client: TxlineClient) {}

  async snapshot(fixtureId: number, asOf?: number): Promise<Scores[]> {
    return await this.client.getJson(`/scores/snapshot/${fixtureId}`, [["asOf", asOf]]);
  }

  async liveUpdatesByFixture(fixtureId: number): Promise<Scores[]> {
    return await this.client.getJson(`/scores/updates/${fixtureId}`);
  }

  async historicalUpdates(options: {
    readonly epochDay: number;
    readonly hourOfDay: number;
    readonly interval: number;
    readonly fixtureId?: number;
  }): Promise<Scores[]> {
    validateHour(options.hourOfDay);
    validateInterval(options.interval);
    return await this.client.getJson(
      `/scores/updates/${options.epochDay}/${options.hourOfDay}/${options.interval}`,
      [["fixtureId", options.fixtureId]],
    );
  }

  async historicalByFixture(fixtureId: number): Promise<Scores[]> {
    return await this.client.getJson(`/scores/historical/${fixtureId}`);
  }

  async statValidationLegacy(options: {
    readonly fixtureId: number;
    readonly seq: number;
    readonly statKey: number;
    readonly statKey2?: number;
  }): Promise<ScoresStatValidation> {
    ensurePositiveSeq(options.seq);
    return await this.client.getJson("/scores/stat-validation", [
      ["fixtureId", options.fixtureId],
      ["seq", options.seq],
      ["statKey", options.statKey],
      ["statKey2", options.statKey2],
    ]);
  }

  async statValidationV2(options: {
    readonly fixtureId: number;
    readonly seq: number;
    readonly statKeys: readonly number[];
  }): Promise<ScoresStatValidationV2> {
    ensurePositiveSeq(options.seq);
    if (options.statKeys.length === 0) {
      throw new InvalidInputError(
        "V2 stat validation requires at least one stat key",
      );
    }
    const statKeysCsv = options.statKeys.join(",");
    const response = await this.client.getJson<ScoresStatValidationV2Response>(
      "/scores/stat-validation",
      [
        ["fixtureId", options.fixtureId],
        ["seq", options.seq],
        ["statKeys", statKeysCsv],
      ],
    );
    return ScoresStatValidationV2.fromResponse([...options.statKeys], response);
  }
}

export function ensurePositiveSeq(seq: number): void {
  if (!Number.isInteger(seq) || seq <= 0) {
    throw new InvalidInputError(
      "score stat validation seq must be greater than zero and must come from a real score record",
    );
  }
}
