import { InvalidInputError } from "../errors.js";
import type { TxlineClient } from "../client.js";
import type { Fixture, FixtureBatchValidation, FixtureValidation } from "./models.js";

export class FixturesClient {
  constructor(private readonly client: TxlineClient) {}

  async snapshot(options: {
    readonly startEpochDay?: number;
    readonly competitionId?: number;
  } = {}): Promise<Fixture[]> {
    return await this.client.getJson("/fixtures/snapshot", [
      ["startEpochDay", options.startEpochDay],
      ["competitionId", options.competitionId],
    ]);
  }

  async updates(epochDay: number, hourOfDay: number): Promise<Fixture[]> {
    validateHour(hourOfDay);
    return await this.client.getJson(`/fixtures/updates/${epochDay}/${hourOfDay}`);
  }

  async validation(
    fixtureId: number,
    timestamp?: number,
  ): Promise<FixtureValidation> {
    return await this.client.getJson("/fixtures/validation", [
      ["fixtureId", fixtureId],
      ["timestamp", timestamp],
    ]);
  }

  async batchValidation(
    epochDay: number,
    hourOfDay: number,
  ): Promise<FixtureBatchValidation> {
    validateHour(hourOfDay);
    return await this.client.getJson("/fixtures/batch-validation", [
      ["epochDay", epochDay],
      ["hourOfDay", hourOfDay],
    ]);
  }
}

export function validateHour(hourOfDay: number): void {
  if (!Number.isInteger(hourOfDay) || hourOfDay < 0 || hourOfDay > 23) {
    throw new InvalidInputError("hourOfDay must be 0..=23");
  }
}

export function validateInterval(interval: number): void {
  if (!Number.isInteger(interval) || interval < 0 || interval > 11) {
    throw new InvalidInputError(
      "interval must be the 0-indexed 5-minute bucket 0..=11",
    );
  }
}
