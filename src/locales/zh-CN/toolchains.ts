export default {
  title: '工具链',
  description: '安装、卸载和管理 Rust 工具链',
  action: {
    installNew: '安装新工具链',
    setDefault: '设为默认',
  },
  status: {
    noToolchains: '尚未安装工具链。',
    installFirst: '请先安装工具链',
    goInstall: '前往安装',
  },
  dialog: {
    installTitle: '安装工具链',
    confirmUninstall: '确认卸载',
    uninstallConfirm: '移除工具链 {name}？',
  },
  form: {
    channel: '频道',
    date: '日期',
    dateOptional: '（可选）',
  },
  channel: {
    stable: 'Stable',
    stableDesc: '最新稳定版',
    beta: 'Beta',
    betaDesc: '最新测试版',
    nightly: 'Nightly',
    nightlyDesc: '每日构建（日期可选）',
  },
  help: {
    dateHelp: '指定特定 nightly 构建的日期（如 2024-01-15）。留空则使用最新 nightly。',
    stableBetaHelp: '{channel} 频道始终安装最新可用版本，无需选择日期。',
  },
  progress: {
    title: '正在安装工具链',
    running: '正在安装 {channel}...',
    success: '{channel} 安装成功',
    failed: '安装失败',
    installing: '安装中...',
  },
}
