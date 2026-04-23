<script lang="ts">
  import { goto } from "$app/navigation";
  import { apiFetch } from "$lib/core/http/api-fetch";
  import { Button } from "$lib/components/Button";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Select from "$lib/components/ui/select";
  import { PasswordInput } from "$lib/components/ui/password-input/index.js";
  import {
    FieldGroup,
    Field,
    FieldLabel,
    FieldDescription,
  } from "$lib/components/ui/field/index.js";

  const DEFAULT_SERVER_ERROR_MESSAGE =
    "Unable to log in right now. Please try again.";

  let username = $state("admin");
  let password = $state("");
  let errorMessage = $state<string | null>(null);
  let submitting = $state(false);
  const loginFormValid = $derived(password.length > 0);

  interface LoginResponse {
    success?: boolean;
    message?: string | null;
  }

  function getSelectPortalTarget(): Element | string {
    if (typeof document === "undefined") return "body";
    const openDialog = document.querySelector("dialog[open]");
    if (openDialog instanceof Element) return openDialog;
    return "body";
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
      const response = await apiFetch("/api/auth/login", {
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

      errorMessage = payload?.message ?? DEFAULT_SERVER_ERROR_MESSAGE;
    } catch {
      errorMessage = DEFAULT_SERVER_ERROR_MESSAGE;
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
          <FieldLabel for="login-username">User</FieldLabel>
          <Select.Root type="single" bind:value={username}>
            <Select.Trigger id="login-username" class="w-full">
              {username === "operator" ? "Operator" : "Admin"}
            </Select.Trigger>
            <Select.Content
              portalProps={{ to: getSelectPortalTarget() }}
              class="z-[80] border border-border bg-card shadow-lg"
            >
              <Select.Group>
                <Select.Item value="admin" label="Admin" />
                <Select.Item value="operator" label="Operator" />
              </Select.Group>
            </Select.Content>
          </Select.Root>
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
          <FieldDescription class="text-destructive"
            >{errorMessage}</FieldDescription
          >
        {/if}
        <Field>
          <Button
            type="submit"
            variant="default"
            label="Login"
            loading={submitting}
            loadingLabel="Logging in..."
            class="w-full"
            disabled={submitting || !loginFormValid}
          />
        </Field>
      </FieldGroup>
    </form>
  </Card.Content>
</Card.Root>
