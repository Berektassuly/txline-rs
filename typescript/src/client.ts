import { ApiToken, AuthHeaders, GuestJwt, activationPreimage } from "./auth.js";
import { devnetConfig, validateConfig, type TxlineConfig } from "./config.js";
import {
  MissingApiTokenError,
  MissingGuestJwtError,
  InvalidInputError,
} from "./errors.js";
import { FixturesClient } from "./http/fixtures.js";
import { OddsClient } from "./http/odds.js";
import {
  buildApiUrl,
  decodeJsonResponse,
  decodeTextResponse,
  statusError,
  type FetchLike,
  type QueryEntries,
} from "./http/request.js";
import { ScoresClient } from "./http/scores.js";
import type { PurchaseQuoteResponse } from "./http/models.js";
import {
  purchaseQuote,
  type AddressLike,
} from "./solana/index.js";
import {
  devnetPurchaseSafetyConfig,
  validatedPurchaseQuote,
  type ValidatedPurchaseQuote,
} from "./solana/transactionSafety.js";
import { OddsStreamClient } from "./stream/odds.js";
import { ScoresStreamClient } from "./stream/scores.js";

export interface TxlineClientOptions {
  readonly config?: TxlineConfig;
  readonly fetch?: FetchLike;
}

interface TokenResponse {
  readonly token: string;
}

export class TxlineClient {
  readonly config: TxlineConfig;
  readonly fetchImpl: FetchLike;
  #guestJwt?: GuestJwt;
  #apiToken?: ApiToken;
  #refreshPromise: Promise<GuestJwt> | undefined;

  constructor(options: TxlineClientOptions = {}) {
    this.config = options.config ?? devnetConfig();
    validateConfig(this.config);
    this.fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  fixtures(): FixturesClient {
    return new FixturesClient(this);
  }

  odds(): OddsClient {
    return new OddsClient(this);
  }

  scores(): ScoresClient {
    return new ScoresClient(this);
  }

  oddsStream(): OddsStreamClient {
    return new OddsStreamClient(this);
  }

  scoresStream(): ScoresStreamClient {
    return new ScoresStreamClient(this);
  }

  async purchaseQuote(
    buyerPubkey: AddressLike,
    txlineAmount: number | bigint,
  ): Promise<PurchaseQuoteResponse> {
    return await purchaseQuote(this, buyerPubkey, txlineAmount);
  }

  async purchaseQuoteChecked(
    buyerPubkey: AddressLike,
    txlineAmount: number | bigint,
    expectedBackendSigner: AddressLike,
  ): Promise<ValidatedPurchaseQuote> {
    const quote = await this.purchaseQuote(buyerPubkey, txlineAmount);
    return await validatedPurchaseQuote(
      quote,
      devnetPurchaseSafetyConfig({
        expectedBuyer: buyerPubkey,
        expectedTxlineAmount: txlineAmount,
        expectedBackendSigner,
      }),
    );
  }

  setGuestJwt(jwt: GuestJwt | string): void {
    this.#guestJwt = typeof jwt === "string" ? new GuestJwt(jwt) : jwt;
  }

  setApiToken(token: ApiToken | string): void {
    this.#apiToken = typeof token === "string" ? new ApiToken(token) : token;
  }

  guestJwt(): GuestJwt | undefined {
    return this.#guestJwt;
  }

  apiToken(): ApiToken | undefined {
    return this.#apiToken;
  }

  authHeaders(requireApiToken: boolean): AuthHeaders {
    const jwt = this.#guestJwt;
    if (!jwt) {
      throw new MissingGuestJwtError();
    }
    const apiToken = this.#apiToken;
    if (requireApiToken && !apiToken) {
      throw new MissingApiTokenError();
    }
    return new AuthHeaders(jwt, apiToken);
  }

  activationPreimage(txSig: string, selectedLeagues: readonly number[]): string {
    const jwt = this.#guestJwt;
    if (!jwt) {
      throw new MissingGuestJwtError();
    }
    return activationPreimage(txSig, selectedLeagues, jwt);
  }

  async startGuestSession(): Promise<GuestJwt> {
    return await this.refreshGuestSession();
  }

  async refreshGuestSessionAfterFailure(staleJwt?: GuestJwt): Promise<GuestJwt> {
    const current = this.#guestJwt;
    if (staleJwt && current && current.asString() !== staleJwt.asString()) {
      return current;
    }
    return await this.refreshGuestSession();
  }

  async activateSubscription(
    txSig: string,
    selectedLeagues: readonly number[],
    walletSignatureBase64: string,
  ): Promise<ApiToken> {
    if (txSig.trim().length === 0) {
      throw new InvalidInputError(
        "subscription transaction signature must not be empty",
      );
    }
    if (walletSignatureBase64.trim().length === 0) {
      throw new InvalidInputError(
        "wallet activation signature must not be empty",
      );
    }
    const response = await this.postJson<string | TokenResponse>(
      "/token/activate",
      {
        txSig,
        walletSignature: walletSignatureBase64,
        leagues: selectedLeagues,
      },
      false,
      "text",
    );
    const tokenValue =
      typeof response === "string" ? parseActivationToken(response) : response.token;
    const token = new ApiToken(tokenValue);
    this.setApiToken(token);
    return token;
  }

  async getJson<T>(
    path: string,
    query: QueryEntries = [],
    requireApiToken = true,
  ): Promise<T> {
    return await this.requestJson<T>("GET", path, query, undefined, requireApiToken);
  }

  async postJson<T>(
    path: string,
    body: unknown,
    requireApiToken = true,
    decodeAs: "json" | "text" = "json",
  ): Promise<T> {
    return await this.requestJson<T>(
      "POST",
      path,
      [],
      body,
      requireApiToken,
      decodeAs,
    );
  }

  async sseResponse(
    path: string,
    query: QueryEntries = [],
    lastEventId?: string,
    signal?: AbortSignal,
  ): Promise<Response> {
    const stale = this.#guestJwt;
    let response = await this.sendSseRequest(path, query, lastEventId, signal);
    if (response.status === 401 || response.status === 403) {
      await this.refreshGuestSessionAfterFailure(stale);
      response = await this.sendSseRequest(path, query, lastEventId, signal);
    }
    if (!response.ok) {
      throw await statusError(response);
    }
    return response;
  }

  private async refreshGuestSession(): Promise<GuestJwt> {
    if (!this.#refreshPromise) {
      this.#refreshPromise = this.fetchImpl(this.config.guestAuthUrl, {
        method: "POST",
        headers: {
          "User-Agent": `txline-ts/0.3.5`,
        },
      })
        .then(decodeJsonResponse<TokenResponse>)
        .then((response) => {
          const token = new GuestJwt(response.token);
          this.setGuestJwt(token);
          return token;
        })
        .finally(() => {
          this.#refreshPromise = undefined;
        });
    }
    return await this.#refreshPromise;
  }

  private async requestJson<T>(
    method: "GET" | "POST",
    path: string,
    query: QueryEntries,
    body: unknown,
    requireApiToken: boolean,
    decodeAs: "json" | "text" = "json",
  ): Promise<T> {
    const stale = this.#guestJwt;
    let response = await this.sendRequest(
      method,
      path,
      query,
      body,
      requireApiToken,
    );
    if (response.status === 401) {
      await this.refreshGuestSessionAfterFailure(stale);
      response = await this.sendRequest(
        method,
        path,
        query,
        body,
        requireApiToken,
      );
    }
    if (decodeAs === "text") {
      return (await decodeTextResponse(response)) as T;
    }
    return await decodeJsonResponse<T>(response);
  }

  private async sendRequest(
    method: "GET" | "POST",
    path: string,
    query: QueryEntries,
    body: unknown,
    requireApiToken: boolean,
  ): Promise<Response> {
    const headers: Record<string, string> = {
      Accept: "application/json",
      ...this.authHeaders(requireApiToken).toHeaders(),
    };
    const init: RequestInit = {
      method,
      headers,
    };
    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
      init.body = JSON.stringify(body, (_key, value) =>
        typeof value === "bigint" ? value.toString() : value,
      );
    }
    return await this.fetchImpl(buildApiUrl(this.config.apiBase, path, query), init);
  }

  private async sendSseRequest(
    path: string,
    query: QueryEntries,
    lastEventId?: string,
    signal?: AbortSignal,
  ): Promise<Response> {
    const headers: Record<string, string> = {
      Accept: "text/event-stream",
      "Cache-Control": "no-cache",
      ...this.authHeaders(true).toHeaders(),
    };
    if (lastEventId) {
      headers["Last-Event-ID"] = lastEventId;
    }
    const init: RequestInit = {
      method: "GET",
      headers,
    };
    if (signal !== undefined) {
      init.signal = signal;
    }
    return await this.fetchImpl(buildApiUrl(this.config.apiBase, path, query), init);
  }
}

function parseActivationToken(body: string): string {
  const trimmed = body.trim();
  if (trimmed.startsWith("{")) {
    const parsed = JSON.parse(trimmed) as TokenResponse;
    return parsed.token;
  }
  return trimmed;
}
