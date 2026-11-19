<script module lang="ts">
  /** Whether a name is Aven itself, which has no site mark to borrow. */
  export const isAven = (name: string) => /^aven\b/iu.test(name.trim());
</script>

<script lang="ts">
  import AvenTile from "$shared/ui/AvenTile";

  let { size = 16 }: { size?: number } = $props();
</script>

<AvenTile {size} />
