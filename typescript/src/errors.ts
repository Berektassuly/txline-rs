export class TxlineSdkError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = new.target.name;
  }
}

export class ConfigError extends TxlineSdkError {}

export class InvalidInputError extends TxlineSdkError {}

export class ProofDecodeError extends TxlineSdkError {}

export class ValidationPayloadError extends TxlineSdkError {}

export class MissingGuestJwtError extends TxlineSdkError {
  constructor() {
    super("missing guest JWT; call startGuestSession or setGuestJwt first");
  }
}

export class MissingApiTokenError extends TxlineSdkError {
  constructor() {
    super("missing API token; activate a subscription or call setApiToken first");
  }
}

export class HttpStatusError extends TxlineSdkError {
  readonly status: number;
  readonly body: string;

  constructor(status: number, body: string) {
    super(`HTTP ${status}: ${sanitizeHttpStatusBody(body)}`);
    this.status = status;
    this.body = body;
  }

  toJSON(): { name: string; status: number; body: string } {
    return {
      name: this.name,
      status: this.status,
      body: sanitizeHttpStatusBody(this.body),
    };
  }
}

export class SolanaSafetyError extends TxlineSdkError {}

export function sanitizeHttpStatusBody(body: string): string {
  return body.length === 0
    ? "response body empty"
    : `response body redacted (${body.length} bytes)`;
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
