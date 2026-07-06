"""SDK exception types."""

from __future__ import annotations

from dataclasses import dataclass


class TxlineError(Exception):
    """Base class for all SDK errors."""


class ConfigError(TxlineError):
    """Configuration is internally inconsistent or unsupported."""


class MissingGuestJwtError(TxlineError):
    """A request requires a guest JWT."""

    def __init__(self) -> None:
        super().__init__("missing guest JWT; call start_guest_session or set_guest_jwt first")


class MissingApiTokenError(TxlineError):
    """A request requires an activated API token."""

    def __init__(self) -> None:
        super().__init__("missing API token; activate a subscription or call set_api_token first")


class InvalidInputError(TxlineError):
    """Local caller input is malformed."""


class ProofDecodeError(TxlineError):
    """A Merkle proof hash could not be decoded."""


class ValidationError(TxlineError):
    """Validation DTO or strategy data is inconsistent."""


class SolanaError(TxlineError):
    """Solana instruction, PDA, or transaction safety error."""


@dataclass(slots=True)
class HttpStatusError(TxlineError):
    """HTTP non-success status.

    The raw response body is retained for programmatic inspection, but formatted
    output redacts it.
    """

    status_code: int
    body: bytes

    def __post_init__(self) -> None:
        Exception.__init__(self, self.status_code, self.body)

    @property
    def text(self) -> str:
        return self.body.decode("utf-8", errors="replace")

    def __str__(self) -> str:
        if not self.body:
            rendered = "response body empty"
        else:
            rendered = f"response body redacted ({len(self.body)} bytes)"
        return f"HTTP {self.status_code}: {rendered}"

    def __repr__(self) -> str:
        return f"HttpStatusError(status_code={self.status_code}, body=<redacted>)"
