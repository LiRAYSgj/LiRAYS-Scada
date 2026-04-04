<script lang="ts">
	import { goto } from "$app/navigation";
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Card from "$lib/components/ui/card/index.js";
	import { PasswordInput } from "$lib/components/ui/password-input/index.js";
	import {
		FieldGroup,
		Field,
		FieldLabel,
		FieldDescription,
	} from "$lib/components/ui/field/index.js";

	let errorMessage = $state<string | null>(null);
	let password = $state("");
	let submitting = $state(false);
	const setupFormValid = $derived(password.length >= 6);

	interface SetupResponse {
		success?: boolean;
		message?: string | null;
	}

	async function submitSetup(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		if (submitting) {
			return;
		}

		errorMessage = null;
		submitting = true;

		try {
			const body = new URLSearchParams({ password });
			const response = await fetch("/auth/setup", {
				method: "POST",
				headers: {
					"content-type": "application/x-www-form-urlencoded",
					accept: "application/json",
				},
				credentials: "include",
				body,
			});

			let payload: SetupResponse | null = null;
			if (response.headers.get("content-type")?.includes("application/json")) {
				payload = (await response.json()) as SetupResponse;
			}

			if (response.ok && payload?.success) {
				await goto("/", { replaceState: true });
				return;
			}

			errorMessage =
				payload?.message ??
				"Unable to complete setup right now. Please try again.";
		} catch {
			errorMessage = "Unable to complete setup right now. Please try again.";
		} finally {
			submitting = false;
		}
	}
</script>

<Card.Root class="mx-auto w-full max-w-sm">
	<Card.Header>
		<Card.Title class="text-2xl mb-4">Set admin password</Card.Title>
		<Card.Description>
			First-time setup. Define credentials for the <strong>admin</strong> user.
		</Card.Description>
	</Card.Header>
	<Card.Content>
		<form method="post" onsubmit={submitSetup}>
			<FieldGroup>
				<Field>
					<FieldLabel for="setup-password">New password</FieldLabel>
					<PasswordInput
						id="setup-password"
						name="password"
						required
						minlength={6}
						autocomplete="new-password"
						bind:value={password}
					/>
				</Field>
				{#if errorMessage}
					<FieldDescription class="text-destructive">{errorMessage}</FieldDescription>
				{/if}
				<Field>
					<Button type="submit" class="w-full" disabled={!setupFormValid || submitting}>
						{submitting ? "Saving..." : "Save and continue"}
					</Button>
				</Field>
			</FieldGroup>
		</form>
	</Card.Content>
</Card.Root>
