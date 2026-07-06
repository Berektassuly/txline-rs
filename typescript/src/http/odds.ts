import type { TxlineClient } from "../client.js";
import { validateHour, validateInterval } from "./fixtures.js";
import type { OddsPayload, OddsValidation } from "./models.js";

export class OddsClient {
  constructor(private readonly client: TxlineClient) {}

  async snapshot(fixtureId: number, asOf?: number): Promise<OddsPayload[]> {
    return await this.client.getJson(`/odds/snapshot/${fixtureId}`, [["asOf", asOf]]);
  }

  async liveUpdatesByFixture(fixtureId: number): Promise<OddsPayload[]> {
    return await this.client.getJson(`/odds/updates/${fixtureId}`);
  }

  async historicalUpdates(options: {
    readonly epochDay: number;
    readonly hourOfDay: number;
    readonly interval: number;
    readonly fixtureId?: number;
  }): Promise<OddsPayload[]> {
    validateHour(options.hourOfDay);
    validateInterval(options.interval);
    return await this.client.getJson(
      `/odds/updates/${options.epochDay}/${options.hourOfDay}/${options.interval}`,
      [["fixtureId", options.fixtureId]],
    );
  }

  async validation(messageId: string, ts: number): Promise<OddsValidation> {
    return await this.client.getJson("/odds/validation", [
      ["messageId", messageId],
      ["ts", ts],
    ]);
  }
}
