import {
  AccountRole,
  address,
  getAddressDecoder,
  getAddressEncoder,
  type AccountMeta,
  type Address,
  type Instruction,
} from "@solana/kit";

export type AddressLike = Address | string;

export type TxlineInstruction = Instruction & {
  readonly programAddress: Address;
  readonly accounts: readonly AccountMeta[];
  readonly data: Uint8Array;
};

const addressEncoder = getAddressEncoder();
const addressDecoder = getAddressDecoder();

export function toAddress(value: AddressLike): Address {
  return typeof value === "string" ? address(value) : value;
}

export function addressBytes(value: AddressLike): Uint8Array {
  return Uint8Array.from(addressEncoder.encode(toAddress(value)));
}

export function addressFromBytes(bytes: Uint8Array | readonly number[]): Address {
  return addressDecoder.decode(Uint8Array.from(bytes));
}

export function writableSigner(value: AddressLike): AccountMeta {
  return { address: toAddress(value), role: AccountRole.WRITABLE_SIGNER };
}

export function readonlySigner(value: AddressLike): AccountMeta {
  return { address: toAddress(value), role: AccountRole.READONLY_SIGNER };
}

export function writable(value: AddressLike): AccountMeta {
  return { address: toAddress(value), role: AccountRole.WRITABLE };
}

export function readonly(value: AddressLike): AccountMeta {
  return { address: toAddress(value), role: AccountRole.READONLY };
}

export function isSignerRole(role: AccountRole): boolean {
  return role === AccountRole.READONLY_SIGNER || role === AccountRole.WRITABLE_SIGNER;
}
