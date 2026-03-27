/**
 * HTTP API client for the branchdeck daemon.
 * All HTTP calls go through this module — components never call fetch directly.
 */

const BASE_URL =
  import.meta.env.VITE_API_URL ?? `${window.location.protocol}//${window.location.host}/api`;

export function getBaseUrl(): string {
  return BASE_URL;
}

async function handleResponse<T>(res: Response, method: string, path: string): Promise<T> {
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(
      `API ${method} ${path} failed: ${res.status} ${res.statusText}${body ? ` — ${body}` : ''}`,
    );
  }
  // 204 No Content — return undefined (callers expecting void)
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  if (!text) return undefined as T;
  return JSON.parse(text) as T;
}

export async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`);
  return handleResponse<T>(res, 'GET', path);
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  return handleResponse<T>(res, 'POST', path);
}

export async function apiPut<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  return handleResponse<T>(res, 'PUT', path);
}

export async function apiDelete<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, { method: 'DELETE' });
  return handleResponse<T>(res, 'DELETE', path);
}
