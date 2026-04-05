<script lang="ts">
  import { Button as UIButton } from "$lib/components/ui/button";
  import * as Avatar from "$lib/components/ui/avatar";
  import { Moon, Play, Sun } from "lucide-svelte";

  type ThemeMode = "light" | "dark";
  type WorkspaceMode = "designer" | "runtime";

  interface Props {
    theme: ThemeMode;
    workspaceMode: WorkspaceMode;
    username: string;
    onSelectWorkspaceMode: (mode: WorkspaceMode) => void;
    onToggleTheme: () => void;
  }

  let {
    theme,
    workspaceMode,
    username,
    onSelectWorkspaceMode,
    onToggleTheme,
  }: Props = $props();

  const avatarInitial = $derived(username.trim().charAt(0).toUpperCase() || "U");
  const runtimeCtaLabel = $derived(
    workspaceMode === "designer" ? "Runtime Mode" : "Designer Mode",
  );
  const themeCtaLabel = $derived(theme === "dark" ? "Light mode" : "Dark mode");
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
    <UIButton
      variant="outline"
      size="sm"
      class="h-8 gap-1.5 border-slate-500/50 px-3.5 text-[12px] font-medium text-slate-700 hover:bg-muted dark:text-muted-foreground"
      title={runtimeCtaLabel}
      onclick={() =>
        onSelectWorkspaceMode(
          workspaceMode === "designer" ? "runtime" : "designer",
        )}
    >
      <Play class="size-3.5" />
      <span>{runtimeCtaLabel}</span>
    </UIButton>

    <UIButton
      variant="outline"
      size="sm"
      class="h-8 gap-1.5 border-slate-500/50 px-3.5 text-[12px] font-medium text-slate-700 hover:bg-muted dark:text-muted-foreground"
      title={themeCtaLabel}
      onclick={onToggleTheme}
    >
      {#if theme === "dark"}
        <Sun class="size-3.5" />
      {:else}
        <Moon class="size-3.5" />
      {/if}
      <span>{themeCtaLabel}</span>
    </UIButton>

    <div class="flex items-center gap-2 rounded-full border border-border bg-muted px-3 py-2">
      <Avatar.Root class="size-6 bg-primary text-primary-foreground">
        <Avatar.Fallback>{avatarInitial}</Avatar.Fallback>
      </Avatar.Root>
      <div class="text-xs font-semibold text-foreground">{username}</div>
    </div>

    <form method="get" action="/auth/logout">
      <UIButton variant="outline" size="sm" class="ml-1" type="submit">Logout</UIButton>
    </form>
  </div>
</div>
