import { HttpStatusError } from "../errors.js";

export type QueryValue = string | number | bigint | boolean | undefined;
export type QueryEntries = readonly (readonly [string, QueryValue])[];

export type FetchLike = (
  input: URL | RequestInfo,
  init?: RequestInit,
) => Promise<Response>;

export function buildApiUrl(
  apiBase: string,
  path: string,
  query: QueryEntries = [],
): URL {
  const normalized = path.startsWith("/") ? path.slice(1) : path;
  const base = apiBase.endsWith("/") ? apiBase : `${apiBase}/`;
  const url = new URL(normalized, base);
  for (const [name, value] of query) {
    if (value !== undefined) {
      url.searchParams.append(name, String(value));
    }
  }
  return url;
}

export async function decodeJsonResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw await statusError(response);
  }
  return (await response.json()) as T;
}

export async function decodeTextResponse(response: Response): Promise<string> {
  if (!response.ok) {
    throw await statusError(response);
  }
  return await response.text();
}

export async function statusError(response: Response): Promise<HttpStatusError> {
  let body = "";
  try {
    body = await response.text();
  } catch {
    body = "";
  }
  return new HttpStatusError(response.status, body);
}
