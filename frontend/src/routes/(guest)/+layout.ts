import { browser } from "$app/environment";
import { redirect } from "@sveltejs/kit";
import type { LayoutLoad } from "./$types";
import { getAuthStatus } from "$lib/auth/session";

export const load: LayoutLoad = async ({ fetch }) => {
	if (!browser) {
		return;
	}

	const status = await getAuthStatus(fetch);
	if (!status) {
		return;
	}

	if (!status.authEnabled || status.authenticated) {
		throw redirect(307, "/");
	}
};
