/**
 * HTTP API client for the branchdeck daemon.
 * All HTTP calls go through this module — components never call fetch directly.
 */

const BASE_URL =
  import.meta.env.VITE_API_URL ?? `${window.location.protocol}//${window.location.host}/api`;

/** Maximum retry attempts for transient errors. */
const MAX_RETRIES = 3;

/** Backoff delays in ms (1s, 2s, 4s). */
const BACKOFF_MS = [1000, 2000, 4000];

/**
 * Check if an HTTP status code is a transient server error worth retrying.
 * Retries on 5xx; never retries on 4xx (client errors are permanent).
 */
function isTransientStatus(status: number): boolean {
  return status >= 500 && status < 600;
}

/**
 * Check if an error is a network-level failure (fetch itself threw).
 */
function isNetworkError(err: unknown): boolean {
  return err instanceof TypeError && (err as TypeError).message.includes('fetch');
}

/**
 * Fetch with retry for transient errors (5xx and network failures).
 * Does NOT retry 4xx errors. Uses exponential backoff: 1s, 2s, 4s.
 */
async function fetchWithRetry(url: string, opts?: RequestInit): Promise<Response> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    try {
      const res = await fetch(url, opts);

      if (res.ok || !isTransientStatus(res.status)) {
        return res;
      }

      // Transient server error — retry if attempts remain
      lastError = new Error(`${res.status} ${res.statusText}`);
    } catch (err) {
      // Network-level error (connection refused, DNS failure, etc.)
      if (!isNetworkError(err)) {
        throw err;
      }
      lastError = err as Error;
    }

    if (attempt < MAX_RETRIES) {
      await new Promise((resolve) => setTimeout(resolve, BACKOFF_MS[attempt]));
    }
  }

  throw lastError ?? new Error('Request failed after retries');
}

export async function apiGet<T>(path: string): Promise<T> {
  const res = await fetchWithRetry(`${BASE_URL}${path}`);
  if (!res.ok) {
    throw new Error(`API GET ${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetchWithRetry(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`API POST ${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

export function getBaseUrl(): string {
  return BASE_URL;
}
