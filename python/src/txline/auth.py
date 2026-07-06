"""Credential wrappers and activation preimage helpers."""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from txline.errors import InvalidInputError

API_TOKEN_HEADER = "X-Api-Token"


@dataclass(frozen=True, slots=True)
class GuestJwt:
    """Guest JWT wrapper with redacted string/debug output."""

    _value: str

    def __init__(self, token: str) -> None:
        value = token.strip()
        if not value:
            raise InvalidInputError("guest JWT must not be empty")
        object.__setattr__(self, "_value", value)

    def as_str(self) -> str:
        return self._value

    def __str__(self) -> str:
        return "GuestJwt(<redacted>)"

    def __repr__(self) -> str:
        return "GuestJwt(<redacted>)"


@dataclass(frozen=True, slots=True)
class ApiToken:
    """Activated API token wrapper with redacted string/debug output."""

    _value: str

    def __init__(self, token: str) -> None:
        value = token.strip()
        if not value:
            raise InvalidInputError("API token must not be empty")
        object.__setattr__(self, "_value", value)

    def as_str(self) -> str:
        return self._value

    def __str__(self) -> str:
        return "ApiToken(<redacted>)"

    def __repr__(self) -> str:
        return "ApiToken(<redacted>)"


@dataclass(frozen=True, slots=True)
class AuthHeaders:
    """Authorization headers for TxLINE requests."""

    authorization: GuestJwt
    api_token: ApiToken | None = None

    def to_headers(self) -> dict[str, str]:
        headers = {"Authorization": f"Bearer {self.authorization.as_str()}"}
        if self.api_token is not None:
            headers[API_TOKEN_HEADER] = self.api_token.as_str()
        return headers

    def has_api_token(self) -> bool:
        return self.api_token is not None

    def __repr__(self) -> str:
        api_token = "<redacted>" if self.api_token is not None else None
        return f"AuthHeaders(authorization=<redacted>, api_token={api_token!r})"


def activation_preimage(
    tx_sig: str, selected_leagues: list[int] | tuple[int, ...], jwt: GuestJwt
) -> str:
    leagues = ",".join(str(league) for league in selected_leagues)
    return f"{tx_sig}:{leagues}:{jwt.as_str()}"


def redact_headers(headers: Mapping[str, str]) -> dict[str, str]:
    redacted = dict(headers)
    for name in ("Authorization", "authorization", API_TOKEN_HEADER):
        if name in redacted:
            redacted[name] = "<redacted>"
    return redacted
