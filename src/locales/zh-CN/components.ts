export default {
  title: '组件',
  description: '管理 Rust 工具链的组件',
  action: {
    installing: '正在安装',
    removing: '正在移除',
  },
  status: {
    selectPrompt: '选择一个工具链并点击"加载"以查看组件。',
  },
  section: {
    installed: '已安装',
    available: '可安装',
  },
  placeholder: {
    search: '搜索组件...',
    toolchainPlaceholder: '选择工具链',
  },
  progress: {
    installTitle: '正在安装组件',
    removeTitle: '正在移除组件',
    running: '{action} {name}...',
    log: '正在为 {toolchain} {action} {name}...',
    success: '{name} 安装/移除成功',
    failed: '{name} 安装/移除失败',
  },
  datePicker: {
    today: '今天',
    clear: '清除',
    selectStartDate: '选择起始日期',
    selectEndDate: '选择结束日期',
  },
}
