<script lang="ts">
  import { onMount } from 'svelte';
  // LiquidGlass-UI（GitHub 安装，见 package.json）：注册 <liquid-glass-*> 自定义元素。
  // 模块自带 SSR 守卫（无 customElements 时不注册），构建/预渲染安全。
  import '@hwyuanzi/liquid-glass-ui';
  import '../app.css';

  // 禁用 WebView 默认右键菜单（弹窗与设置窗口均生效）
  onMount(() => {
    const prevent = (e: Event) => e.preventDefault();
    window.addEventListener('contextmenu', prevent);
    return () => window.removeEventListener('contextmenu', prevent);
  });
</script>

<slot />
