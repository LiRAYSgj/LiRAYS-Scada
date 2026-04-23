import { apiFetch } from "$lib/core/http/api-fetch";

export type AuthUserName = "admin" | "operator";
export type AuthUserRole = "admin" | "operator";

export interface AuthStatus {
  authEnabled: boolean;
  authenticated: boolean;
  adminExists: boolean;
  user: {
    username: AuthUserName;
    role: AuthUserRole;
  } | null;
}

function isKnownUserLiteral(
  value: unknown,
): value is AuthUserName | AuthUserRole {
  return value === "admin" || value === "operator";
}

function isAuthStatus(value: unknown): value is AuthStatus {
  if (!value || typeof value !== "object") {
    return false;
  }

  const maybe = value as Record<string, unknown>;
  const user = maybe.user;
  const hasValidUser =
    user === null ||
    (typeof user === "object" &&
      user !== null &&
      isKnownUserLiteral((user as Record<string, unknown>).username) &&
      isKnownUserLiteral((user as Record<string, unknown>).role));

  return (
    typeof maybe.authEnabled === "boolean" &&
    typeof maybe.authenticated === "boolean" &&
    typeof maybe.adminExists === "boolean" &&
    hasValidUser
  );
}

export async function getAuthStatus(
  fetchFn: typeof fetch,
): Promise<AuthStatus | null> {
  try {
    const response = await apiFetch(
      "/api/auth/status",
      {
        method: "GET",
        credentials: "include",
        headers: {
          accept: "application/json",
        },
        cache: "no-store",
      },
      fetchFn,
    );

    if (!response.ok) {
      return null;
    }

    const payload = (await response.json()) as unknown;
    if (!isAuthStatus(payload)) {
      return null;
    }

    return payload;
  } catch {
    return null;
  }
}
