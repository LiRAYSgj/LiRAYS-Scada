import { browser } from "$app/environment";
import { redirect } from "@sveltejs/kit";
import type { LayoutLoad } from "./$types";
import { getAuthStatus } from "$lib/auth/session";

export const load: LayoutLoad = async ({ fetch }) => {
  if (!browser) {
    return {
      authEnabled: true,
      username: "admin" as const,
      role: "admin" as const,
    };
  }

  const status = await getAuthStatus(fetch);
  if (!status) {
    throw redirect(307, "/auth/login");
  }

  if (!status.authEnabled) {
    return {
      authEnabled: false,
      username: "admin" as const,
      role: "admin" as const,
    };
  }

  if (!status.adminExists) {
    throw redirect(307, "/auth/setup");
  }

  if (!status.authenticated) {
    throw redirect(307, "/auth/login");
  }

  return {
    authEnabled: true,
    username: status.user?.username ?? "admin",
    role: status.user?.role ?? "admin",
  };
};
