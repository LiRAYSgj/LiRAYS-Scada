import { env } from "$env/dynamic/private";
import type { RequestEvent } from "@sveltejs/kit";

const DEFAULT_BACKEND_ORIGIN = "http://127.0.0.1:8245";

function getBackendOrigin(): string {
  return (
    env.BACKEND_ORIGIN ??
    env.SCADA_BACKEND_ORIGIN ??
    DEFAULT_BACKEND_ORIGIN
  );
}

export async function forwardAuthRequest(
  event: RequestEvent,
  backendPath: string,
  method: "GET" | "POST",
): Promise<Response> {
  const backendOrigin = getBackendOrigin();
  const headers = new Headers();
  const cookie = event.request.headers.get("cookie");
  const accept = event.request.headers.get("accept");
  const contentType = event.request.headers.get("content-type");

  if (cookie) headers.set("cookie", cookie);
  if (accept) headers.set("accept", accept);
  if (contentType) headers.set("content-type", contentType);

  const init: RequestInit = {
    method,
    headers,
    redirect: "manual",
  };

  if (method === "POST") {
    init.body = await event.request.text();
  }

  const upstream = await fetch(`${backendOrigin}${backendPath}`, init);
  return new Response(upstream.body, {
    status: upstream.status,
    statusText: upstream.statusText,
    headers: new Headers(upstream.headers),
  });
}
