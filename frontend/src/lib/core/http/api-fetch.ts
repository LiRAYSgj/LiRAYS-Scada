import { browser } from "$app/environment";

const LOGIN_PATH = "/auth/login";

let redirectingToLogin = false;

function resolvePathname(input: RequestInfo | URL): string | null {
  if (typeof input === "string") {
    if (input.startsWith("/")) {
      return input;
    }

    try {
      return new URL(input).pathname;
    } catch {
      return null;
    }
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  if (input instanceof Request) {
    try {
      return new URL(input.url).pathname;
    } catch {
      return null;
    }
  }

  return null;
}

function isApiPath(pathname: string | null): boolean {
  if (!pathname) {
    return false;
  }

  return pathname === "/api" || pathname.startsWith("/api/");
}

function redirectToLogin(): void {
  if (!browser || redirectingToLogin) {
    return;
  }

  if (window.location.pathname === LOGIN_PATH) {
    return;
  }

  redirectingToLogin = true;

  window.location.replace(LOGIN_PATH);
}

export async function apiFetch(
  input: RequestInfo | URL,
  init?: RequestInit,
  fetchFn: typeof fetch = fetch,
): Promise<Response> {
  const response = await fetchFn(input, init);

  if (browser && response.status === 401 && isApiPath(resolvePathname(input))) {
    redirectToLogin();
  }

  return response;
}
