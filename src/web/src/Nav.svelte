<script lang="ts">
  // The view switcher shared by both pages: one tap between the Davis floor and the campus.
  let { current }: { current: 'floor' | 'campus' | 'phone' } = $props();
  const views = [
    { id: 'floor', label: 'Davis floor', href: './' },
    { id: 'campus', label: 'Campus', href: 'campus.html' },
    { id: 'phone', label: 'Sensor test', href: 'phone.html' },
  ] as const;
</script>

<nav class="switcher" aria-label="Views">
  {#each views as v (v.id)}
    <a href={v.href} aria-current={v.id === current ? 'page' : undefined}>{v.label}</a>
  {/each}
</nav>

<style>
  .switcher {
    position: fixed;
    top: var(--gutter);
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem;
    border: 1px solid var(--hairline);
    border-radius: 999px;
    background: var(--panel);
    backdrop-filter: blur(18px) saturate(140%);
    -webkit-backdrop-filter: blur(18px) saturate(140%);
    pointer-events: auto;
    z-index: 5;
  }
  a {
    display: inline-grid;
    place-items: center;
    min-height: var(--tap);
    padding: 0 1.15rem;
    border-radius: 999px;
    color: var(--ink-2);
    text-decoration: none;
    font-weight: 550;
    letter-spacing: 0.01em;
    white-space: nowrap;
    transition:
      background-color 160ms var(--ease-out),
      color 160ms var(--ease-out);
  }
  a[aria-current='page'] {
    background: var(--accent);
    color: var(--accent-ink);
  }
  @media (hover: hover) and (pointer: fine) {
    a:not([aria-current='page']):hover {
      background: var(--wash);
      color: var(--ink);
    }
  }
</style>
