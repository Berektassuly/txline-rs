import { ProofDecodeError } from "../errors.js";

export type Hash32Like = string | readonly number[] | Uint8Array;

export interface ProofNode {
  readonly hash: Hash32Like;
  readonly isRightSibling: boolean;
}

export interface NormalizedProofNode {
  readonly hash: Uint8Array;
  readonly isRightSibling: boolean;
}

export function decodeHash32(value: Hash32Like): Uint8Array {
  const bytes =
    typeof value === "string" ? decodeHashString(value) : Uint8Array.from(value);
  if (bytes.length !== 32) {
    throw new ProofDecodeError(`expected 32 bytes, received ${bytes.length}`);
  }
  return bytes;
}

export function normalizeProofNode(node: ProofNode): NormalizedProofNode {
  return {
    hash: decodeHash32(node.hash),
    isRightSibling: node.isRightSibling,
  };
}

function decodeHashString(value: string): Uint8Array {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    throw new ProofDecodeError("hash string must not be empty");
  }
  const hexCandidate = trimmed.startsWith("0x") ? trimmed.slice(2) : trimmed;
  if (/^[0-9a-fA-F]{64}$/u.test(hexCandidate)) {
    return decodeHex(hexCandidate);
  }
  return decodeBase64(trimmed);
}

export function decodeHex(value: string): Uint8Array {
  if (value.length % 2 !== 0) {
    throw new ProofDecodeError("hex string must have an even length");
  }
  const out = new Uint8Array(value.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    const byte = Number.parseInt(value.slice(i * 2, i * 2 + 2), 16);
    if (Number.isNaN(byte)) {
      throw new ProofDecodeError("hex string contains a non-hex byte");
    }
    out[i] = byte;
  }
  return out;
}

export function decodeBase64(value: string): Uint8Array {
  const normalized = value.replace(/-/gu, "+").replace(/_/gu, "/");
  const padded = normalized.padEnd(
    normalized.length + ((4 - (normalized.length % 4)) % 4),
    "=",
  );
  try {
    const binary = globalThis.atob(padded);
    return Uint8Array.from(binary, (char) => char.charCodeAt(0));
  } catch (cause) {
    throw new ProofDecodeError("base64 decode error", { cause });
  }
}
