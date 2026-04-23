<script lang="ts">
  import { Button } from "$lib/components/Button";
  import * as Avatar from "$lib/components/ui/avatar";
  import { Moon, Play, Sun } from "lucide-svelte";
  import { goto } from "$app/navigation";
  import { apiFetch } from "$lib/core/http/api-fetch";

  type ThemeMode = "light" | "dark";
  type WorkspaceMode = "designer" | "runtime";

  interface Props {
    theme: ThemeMode;
    workspaceMode: WorkspaceMode;
    username: string;
    authEnabled: boolean;
    showWorkspaceModeSwitch?: boolean;
    onSelectWorkspaceMode: (mode: WorkspaceMode) => void;
    onToggleTheme: () => void;
  }

  let {
    theme,
    workspaceMode,
    username,
    authEnabled,
    showWorkspaceModeSwitch = true,
    onSelectWorkspaceMode,
    onToggleTheme,
  }: Props = $props();

  function toTitleCase(value: string): string {
    const normalized = value.trim().toLowerCase();
    if (!normalized) {
      return "User";
    }

    return normalized
      .split(/\s+/)
      .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
      .join(" ");
  }

  const displayUsername = $derived(toTitleCase(username));
  const avatarInitial = $derived(displayUsername.charAt(0).toUpperCase() || "U");
  const runtimeCtaLabel = $derived(
    workspaceMode === "designer" ? "Runtime Mode" : "Designer Mode",
  );
  const themeCtaLabel = $derived(theme === "dark" ? "Light mode" : "Dark mode");
  
	let errorMessage = $state<string | null>(null);
	let submitting = $state(false);

	async function submitLogout(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		if (submitting) {
			return;
		}

		errorMessage = null;
		submitting = true;

		try {
			const response = await apiFetch("/api/auth/logout", {
				method: "GET",
				headers: {
					"content-type": "application/x-www-form-urlencoded",
					accept: "application/json",
				},
				credentials: "include"
			});

			if (response.ok) {
				await goto("/auth/login", { replaceState: true });
				return;
			}
		} catch {
			errorMessage = "Unable to log out right now. Please try again.";
		} finally {
			submitting = false;
		}
	}
</script>

<div class="flex min-h-[54px] items-center justify-between rounded-md border border-border bg-card px-3 py-2.5 shadow-sm">
  <div class="flex items-end gap-2.5">
    <img
      src="/logo.svg"
      alt="LiRAYS"
      class="h-8 w-auto select-none object-contain"
      draggable="false"
    />
    <h1 class="pb-[2px] text-xl leading-none font-semibold tracking-wide text-foreground">
      SCADA
    </h1>
    <h1 class="sr-only">LiRAYS SCADA</h1>
  </div>

  <div class="ml-auto flex items-center gap-3">
    {#if showWorkspaceModeSwitch}
      <Button
        variant="outline"
        size="sm"
        icon={Play}
        label={runtimeCtaLabel}
        class="border-slate-500/50 text-slate-700 hover:bg-muted dark:text-muted-foreground"
        title={runtimeCtaLabel}
        onclick={() =>
          onSelectWorkspaceMode(
            workspaceMode === "designer" ? "runtime" : "designer",
          )}
      />
    {/if}

    <Button
      variant="outline"
      size="sm"
      icon={theme === "dark" ? Sun : Moon}
      label={themeCtaLabel}
      class="border-slate-500/50 text-slate-700 hover:bg-muted dark:text-muted-foreground"
      title={themeCtaLabel}
      onclick={onToggleTheme}
    />

    <div class="flex items-center gap-2 rounded-full border border-border bg-muted px-3 py-2">
      <Avatar.Root class="size-6 bg-primary text-primary-foreground">
        <Avatar.Fallback>{avatarInitial}</Avatar.Fallback>
      </Avatar.Root>
      <div class="text-xs font-semibold text-foreground">{displayUsername}</div>
    </div>

    {#if authEnabled}
      <form method="get" onsubmit={submitLogout}>
        <Button variant="outline" size="sm" class="ml-1" type="submit" label="Logout" />
      </form>
    {/if}
  </div>
</div>
