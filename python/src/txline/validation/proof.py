"""Proof hash decoding helpers."""

from __future__ import annotations

import base64
from collections.abc import Iterable
from dataclasses import dataclass
from typing import Any

from txline.errors import ProofDecodeError


@dataclass(frozen=True, slots=True)
class Hash32:
    """A proof hash that must be exactly 32 bytes."""

    _bytes: bytes

    def __init__(self, value: bytes | bytearray | Iterable[int]) -> None:
        raw = bytes(value)
        if len(raw) != 32:
            raise ProofDecodeError(f"expected 32 bytes, received {len(raw)}")
        object.__setattr__(self, "_bytes", raw)

    @classmethod
    def decode(cls, value: str | bytes | bytearray | Iterable[int]) -> Hash32:
        if isinstance(value, str):
            text = value.strip()
            if not text:
                raise ProofDecodeError("hash string must not be empty")
            hex_candidate = text[2:] if text.startswith("0x") else text
            if len(hex_candidate) == 64 and all(
                char in "0123456789abcdefABCDEF" for char in hex_candidate
            ):
                return cls(bytes.fromhex(hex_candidate))
            for decoder in (
                base64.b64decode,
                base64.urlsafe_b64decode,
                _urlsafe_no_pad_decode,
            ):
                try:
                    return cls(decoder(text))
                except Exception:
                    continue
            raise ProofDecodeError("hash string is not valid base64, URL-safe base64, or hex")
        return cls(value)

    def as_bytes(self) -> bytes:
        return self._bytes

    def __bytes__(self) -> bytes:
        return self._bytes

    def __repr__(self) -> str:
        return f"Hash32(0x{self._bytes.hex()})"


def _urlsafe_no_pad_decode(value: str) -> bytes:
    padding = "=" * (-len(value) % 4)
    return base64.urlsafe_b64decode(value + padding)


@dataclass(frozen=True, slots=True)
class ProofNode:
    hash: Hash32
    is_right_sibling: bool

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ProofNode:
        return cls(
            hash=Hash32.decode(data["hash"]),
            is_right_sibling=bool(data.get("isRightSibling", data.get("is_right_sibling"))),
        )

    def anchor_hash(self) -> bytes:
        return self.hash.as_bytes()


def proof_nodes_from_json(values: Iterable[dict[str, Any]] | None) -> list[ProofNode]:
    return [ProofNode.from_dict(value) for value in values or []]
