import type { ScadaView, ViewInputPayload, ViewsPage } from "../types";

interface ApiEnvelope<T> {
  success: boolean;
  data?: T;
  message?: string;
}

async function parseEnvelope<T>(response: Response): Promise<ApiEnvelope<T>> {
  try {
    return (await response.json()) as ApiEnvelope<T>;
  } catch {
    throw new Error("Invalid server response");
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "include",
    headers: {
      "content-type": "application/json",
      accept: "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });
  const envelope = await parseEnvelope<T>(response);
  if (
    !response.ok ||
    envelope.success === false ||
    envelope.data === undefined
  ) {
    throw new Error(
      envelope.message || `Request failed: ${response.status} ${response.statusText}`,
    );
  }
  return envelope.data;
}

async function requestEmpty(path: string, init?: RequestInit): Promise<void> {
  const response = await fetch(path, {
    credentials: "include",
    headers: {
      accept: "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });
  if (response.status === 204) return;
  const envelope = await parseEnvelope<unknown>(response);
  if (!response.ok || envelope.success === false) {
    throw new Error(
      envelope.message || `Request failed: ${response.status} ${response.statusText}`,
    );
  }
}

export interface ListViewsParams {
  page: number;
  pageSize: number;
  sortBy: "name" | "updated_at" | "is_entry_point";
  sortDirection: "asc" | "desc";
  search?: string;
}

export async function listViews(params: ListViewsParams): Promise<ViewsPage> {
  const search = new URLSearchParams({
    page: String(params.page),
    page_size: String(params.pageSize),
    sort_by: params.sortBy,
    sort_direction: params.sortDirection,
  });
  const query = params.search?.trim();
  if (query) {
    search.set("search", query);
  }
  return requestJson<ViewsPage>(`/api/views?${search.toString()}`, { method: "GET" });
}

export async function getView(id: string): Promise<ScadaView> {
  return requestJson<ScadaView>(`/api/views/${id}`, { method: "GET" });
}

export async function createView(input: ViewInputPayload): Promise<ScadaView> {
  return requestJson<ScadaView>("/api/views", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function updateView(
  id: string,
  input: ViewInputPayload,
): Promise<ScadaView> {
  return requestJson<ScadaView>(`/api/views/${id}`, {
    method: "PUT",
    body: JSON.stringify(input),
  });
}

export async function deleteView(id: string): Promise<void> {
  return requestEmpty(`/api/views/${id}`, { method: "DELETE" });
}

export async function getEntryPointView(): Promise<ScadaView> {
  return requestJson<ScadaView>("/api/views/entry-point", { method: "GET" });
}

export async function setEntryPointView(id: string): Promise<ScadaView> {
  return requestJson<ScadaView>(`/api/views/${id}/entry-point`, {
    method: "PUT",
  });
}
