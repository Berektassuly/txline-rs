import { InvalidInputError } from "./errors.js";

export const API_TOKEN_HEADER = "X-Api-Token";

const INSPECT_SYMBOL = Symbol.for("nodejs.util.inspect.custom");

export class GuestJwt {
  readonly #token: string;

  constructor(token: string) {
    const trimmed = token.trim();
    if (trimmed.length === 0) {
      throw new InvalidInputError("guest JWT must not be empty");
    }
    this.#token = trimmed;
  }

  asString(): string {
    return this.#token;
  }

  toString(): string {
    return "GuestJwt(<redacted>)";
  }

  toJSON(): string {
    return "<redacted>";
  }

  [INSPECT_SYMBOL](): string {
    return this.toString();
  }
}

export class ApiToken {
  readonly #token: string;

  constructor(token: string) {
    const trimmed = token.trim();
    if (trimmed.length === 0) {
      throw new InvalidInputError("API token must not be empty");
    }
    this.#token = trimmed;
  }

  asString(): string {
    return this.#token;
  }

  toString(): string {
    return "ApiToken(<redacted>)";
  }

  toJSON(): string {
    return "<redacted>";
  }

  [INSPECT_SYMBOL](): string {
    return this.toString();
  }
}

export interface GuestSession {
  readonly token: GuestJwt;
}

export class AuthHeaders {
  constructor(
    readonly authorization: GuestJwt,
    readonly apiToken?: ApiToken,
  ) {}

  hasApiToken(): boolean {
    return this.apiToken !== undefined;
  }

  toHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.authorization.asString()}`,
    };
    if (this.apiToken) {
      headers[API_TOKEN_HEADER] = this.apiToken.asString();
    }
    return headers;
  }

  toString(): string {
    return "AuthHeaders(<redacted>)";
  }

  toJSON(): Record<string, string> {
    return {
      authorization: "<redacted>",
      ...(this.apiToken ? { apiToken: "<redacted>" } : {}),
    };
  }

  [INSPECT_SYMBOL](): Record<string, string> {
    return this.toJSON();
  }
}

export function activationPreimage(
  txSig: string,
  selectedLeagues: readonly number[],
  jwt: GuestJwt,
): string {
  return `${txSig}:${selectedLeagues.join(",")}:${jwt.asString()}`;
}

export interface ActivationPayload {
  readonly txSig: string;
  readonly walletSignature: string;
  readonly leagues: readonly number[];
}
