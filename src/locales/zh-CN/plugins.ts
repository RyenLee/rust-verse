export default {
  title: 'Cargo 插件',
  description: '搜索、安装和管理 Cargo 插件',
  section: {
    searchCrates: '在 crates.io 上搜索',
    installedPlugins: '已安装插件',
    installByName: '按 crate 名称安装',
  },
  placeholder: {
    search: '搜索插件（如 cargo-audit）...',
    installByName: '按 crate 名称安装（如 cargo-audit）...',
    filter: '筛选已安装...',
  },
  status: {
    serverError: 'crates.io 服务器错误，请稍后重试。',
    noFilterMatch: '没有匹配的插件。',
    noPlugins: '未安装 Cargo 插件。',
  },
  badge: {
    official: '官方',
  },
  dialog: {
    confirmUninstall: '确认卸载',
    uninstallConfirm: '移除 {name}？',
  },
  progress: {
    title: '正在安装插件',
    running: '正在安装 {name}...',
    success: '{name} 安装成功',
    failed: '{name} 安装失败',
  },
}
