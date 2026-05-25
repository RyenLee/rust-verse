export default {
  title: '历史版本',
  description: '浏览和安装 Rust 工具链的历史版本',
  descriptionSelect: '选择一个历史版本，返回安装页面进行安装',
  action: {
    sync: '同步版本数据',
    syncing: '同步中...',
    searchPlaceholder: '搜索版本号或日期...',
    backToToolchains: '返回工具链',
    select: '选择',
  },
  filter: {
    dateFrom: '起始日期',
    dateTo: '结束日期',
    to: '至',
    dateRange: '选择日期范围',
  },
  channel: {
    stable: 'Stable',
    beta: 'Beta',
    nightly: 'Nightly',
  },
  status: {
    installed: '已安装',
    noData: '暂无版本数据，请先点击"同步版本数据"获取。',
  },
  error: {
    syncHint: '请检查网络连接后重试，或切换到其他频道。',
  },
  progress: {
    title: '正在安装历史版本',
    running: '正在安装 {version}...',
    success: '{version} 安装成功',
    failed: '安装失败',
  },
}
