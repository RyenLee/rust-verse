export default {
  title: '编译目标',
  description: '管理 Rust 工具链的编译目标',
  action: {
    installing: '正在安装',
    removing: '正在移除',
  },
  status: {
    selectPrompt: '选择一个工具链并点击"加载"以查看编译目标。',
  },
  placeholder: {
    search: '搜索编译目标...',
    toolchainPlaceholder: '选择工具链',
  },
  progress: {
    installTitle: '正在安装编译目标',
    removeTitle: '正在移除编译目标',
    running: '{action} {name}...',
    log: '正在为 {toolchain} {action} {name}...',
    success: '{name} 安装/移除成功',
    failed: '{name} 安装/移除失败',
  },
}
