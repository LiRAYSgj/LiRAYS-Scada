import { browser } from "$app/environment";
import { redirect } from "@sveltejs/kit";
import type { PageLoad } from "./$types";
import { getAuthStatus } from "$lib/auth/session";

export const load: PageLoad = async ({ fetch }) => {
	if (!browser) {
		return;
	}

	const status = await getAuthStatus(fetch);
	if (!status || !status.authEnabled) {
		return;
	}

	if (status.adminExists) {
		throw redirect(307, "/auth/login");
	}
};
