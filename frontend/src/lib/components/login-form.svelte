<script lang="ts">
	import { goto } from "$app/navigation";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Card from "$lib/components/ui/card/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { PasswordInput } from "$lib/components/ui/password-input/index.js";
	import {
		FieldGroup,
		Field,
		FieldLabel,
		FieldDescription,
	} from "$lib/components/ui/field/index.js";

	let username = $state("admin");
	let password = $state("");
	let errorMessage = $state<string | null>(null);
	let submitting = $state(false);
	const loginFormValid = $derived(
		username.trim().length > 0 && password.length > 0,
	);

	interface LoginResponse {
		success?: boolean;
		message?: string | null;
	}

	async function submitLogin(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		if (submitting) {
			return;
		}

		errorMessage = null;
		submitting = true;

		try {
			const body = new URLSearchParams({
				username,
				password,
			});
			const response = await fetch("/auth/login", {
				method: "POST",
				headers: {
					"content-type": "application/x-www-form-urlencoded",
					accept: "application/json",
				},
				credentials: "include",
				body,
			});

			let payload: LoginResponse | null = null;
			if (response.headers.get("content-type")?.includes("application/json")) {
				payload = (await response.json()) as LoginResponse;
			}

			if (response.ok && payload?.success) {
				await goto("/", { replaceState: true });
				return;
			}

			if(payload?.message) errorMessage = payload?.message;
		} catch {
			errorMessage = "Unable to log in right now. Please try again.";
		} finally {
			submitting = false;
		}
	}
</script>

<Card.Root class="mx-auto w-full max-w-sm">
	<Card.Header>
		<Card.Title class="text-2xl mb-4">Sign in</Card.Title>
	</Card.Header>
	<Card.Content>
		<form method="post" onsubmit={submitLogin}>
			<FieldGroup>
				<Field>
					<FieldLabel for="login-username">Username</FieldLabel>
					<Input
						id="login-username"
						name="username"
						type="text"
						placeholder="admin"
						required
						autocomplete="username"
						bind:value={username}
					/>
				</Field>
				<Field>
					<FieldLabel for="login-password">Password</FieldLabel>
					<PasswordInput
						id="login-password"
						name="password"
						required
						autocomplete="current-password"
						bind:value={password}
					/>
				</Field>
				{#if errorMessage}
					<FieldDescription class="text-destructive">{errorMessage}</FieldDescription>
				{/if}
				<Field>
					<Button type="submit" class="w-full" disabled={submitting || !loginFormValid}>
						{submitting ? "Logging in..." : "Login"}
					</Button>
				</Field>
			</FieldGroup>
		</form>
	</Card.Content>
</Card.Root>
