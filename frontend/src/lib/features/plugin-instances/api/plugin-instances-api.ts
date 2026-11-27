import { apiFetch } from "$lib/core/http/api-fetch";
import type {
  PluginCollection,
  PluginInstance,
  PluginInstanceInput,
  PluginInstanceUpdate,
} from "../types";

interface ApiEnvelope<T> {
  success: boolean;
  data?: T;
  message?: string;
}

async function request<T>(
  path: string,
  init?: RequestInit,
): Promise<T | undefined> {
  const response = await apiFetch(path, {
    credentials: "include",
    ...init,
    headers: {
      "content-type": "application/json",
      accept: "application/json",
      ...(init?.headers ?? {}),
    },
  });
  let envelope: ApiEnvelope<T>;
  try {
    envelope = (await response.json()) as ApiEnvelope<T>;
  } catch {
    throw new Error(
      `Request failed: ${response.status} ${response.statusText}`,
    );
  }
  if (!response.ok || envelope.success === false) {
    throw new Error(
      envelope.message ||
        `Request failed: ${response.status} ${response.statusText}`,
    );
  }
  return envelope.data;
}

async function requestData<T>(path: string, init?: RequestInit): Promise<T> {
  const data = await request<T>(path, init);
  if (data === undefined) {
    throw new Error("Invalid server response");
  }
  return data;
}

export function listPluginCollections(): Promise<PluginCollection[]> {
  return requestData<PluginCollection[]>("/api/plugin-collections");
}

export function listPluginInstances(
  basePath: string,
): Promise<PluginInstance[]> {
  return requestData<PluginInstance[]>(basePath);
}

export function createPluginInstance(
  basePath: string,
  input: PluginInstanceInput,
): Promise<PluginInstance> {
  return requestData<PluginInstance>(basePath, {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function updatePluginInstance(
  basePath: string,
  id: string,
  update: PluginInstanceUpdate,
): Promise<PluginInstance> {
  return requestData<PluginInstance>(`${basePath}/${encodeURIComponent(id)}`, {
    method: "PUT",
    body: JSON.stringify(update),
  });
}

export async function deletePluginInstance(
  basePath: string,
  id: string,
): Promise<void> {
  await request<unknown>(`${basePath}/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}
