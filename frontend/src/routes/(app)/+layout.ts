import { browser } from "$app/environment";
import { redirect } from "@sveltejs/kit";
import type { LayoutLoad } from "./$types";
import { getAuthStatus } from "$lib/auth/session";
import { getEntryPointView } from "$lib/features/views/api/views-api";

async function resolveRuntimeTarget(): Promise<string | null> {
  try {
    const entryPoint = await getEntryPointView();
    return `/runtime/${entryPoint.id}`;
  } catch {
    return null;
  }
}

export const load: LayoutLoad = async ({ fetch }) => {
  if (!browser) {
    return { authEnabled: true, username: "admin", role: "admin" as const };
  }

  const status = await getAuthStatus(fetch);
  if (!status) {
    throw redirect(307, "/auth/login");
  }

  if (!status.authEnabled) {
    return { authEnabled: false, username: null, role: null };
  }

  if (!status.adminExists) {
    throw redirect(307, "/auth/setup");
  }

  if (!status.authenticated) {
    throw redirect(307, "/auth/login");
  }

  if (status.user?.role === "operator") {
    const runtimeTarget = await resolveRuntimeTarget();
    throw redirect(307, runtimeTarget ?? "/runtime/no-entry");
  }

  return {
    authEnabled: true,
    username: status.user?.username ?? "admin",
    role: status.user?.role ?? "admin",
  };
};
