export interface AuthStatus {
	authEnabled: boolean;
	authenticated: boolean;
	adminExists: boolean;
}

function isAuthStatus(value: unknown): value is AuthStatus {
	if (!value || typeof value !== "object") {
		return false;
	}

	const maybe = value as Record<string, unknown>;
	return (
		typeof maybe.authEnabled === "boolean" &&
		typeof maybe.authenticated === "boolean" &&
		typeof maybe.adminExists === "boolean"
	);
}

export async function getAuthStatus(fetchFn: typeof fetch): Promise<AuthStatus | null> {
	try {
		const response = await fetchFn("/auth/status", {
			method: "GET",
			credentials: "include",
			headers: {
				accept: "application/json",
			},
			cache: "no-store",
		});

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
