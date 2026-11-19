<script lang="ts">
  /** Aven's app icon itself, its own squircle and shadow included, the
   *  same in either theme. */
  let { size = 16 }: { size?: number } = $props();
</script>

<img
  class="aven-tile"
  src="/aven-icon.png"
  srcset="/aven-icon.png 128w, /aven-icon@2x.png 256w"
  sizes={`${size}px`}
  width={size}
  height={size}
  alt=""
  aria-hidden="true"
  draggable="false"
/>

<style>
  .aven-tile {
    display: inline-block;
    flex: none;
    object-fit: contain;
  }
</style>
