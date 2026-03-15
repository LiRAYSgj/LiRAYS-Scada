<script lang="ts">
	import { onMount } from "svelte";
	import "./layout.css";
	import favicon from "$lib/assets/favicon.svg";
	import Snackbar from "$lib/components/Snackbar/Snackbar.svelte";
	import { initThemeFromStorage } from "$lib/stores/theme";

	let { children } = $props();

	onMount(() => {
		initThemeFromStorage();
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
{@render children()}
<Snackbar />
