<script lang="ts">
  import SiteTuning from "./SiteTuning.svelte";
  import VaultFill from "./VaultFill.svelte";

  /** The page's address, its tab, and whether the site menu is open. */
  let { url, tab, open }: { url: string | null; tab: string | null; open: boolean } = $props();
</script>

<SiteTuning {url} {tab} />
<VaultFill {url} {open} />
