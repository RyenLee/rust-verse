export default {
  title: '仪表盘',
  description: 'Rust 开发环境概览',
  rustNotFound: '未找到 Rust 工具链',
  rustNotFoundDesc: '本应用需要 {rustup} 来管理 Rust 工具链。',
  action: {
    installRustup: '安装 rustup',
    viewGuide: '查看安装指南',
    manageToolchains: '管理已安装的工具链',
    checkUpdates: '检查并应用更新',
    addRemoveComponents: '添加或移除组件',
    uninstallRustupDesc: '从系统中完全移除 rustup 及所有工具链',
  },
  status: {
    afterInstall: '安装完成后，请重启本应用。',
    upToDate: '已是最新',
    updatesAvailable: '{count} 个可用',
    ready: '就绪',
    networkError: '无法连接网络 · 更新检查跳过',
  },
  card: {
    defaultToolchain: '默认工具链',
    installed: '已安装',
    updates: '更新',
    environment: '环境',
  },
  section: {
    versions: '版本信息',
    quickActions: '快捷操作',
  },
  label: {
    rustup: 'rustup',
    cargo: 'cargo',
    componentsTargets: '组件与编译目标',
  },
}