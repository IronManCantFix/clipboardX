<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';

  // 禁用 WebView 默认右键菜单（弹窗与设置窗口均生效）
  onMount(() => {
    const prevent = (e: Event) => e.preventDefault();
    window.addEventListener('contextmenu', prevent);
    return () => window.removeEventListener('contextmenu', prevent);
  });
</script>

<slot />
