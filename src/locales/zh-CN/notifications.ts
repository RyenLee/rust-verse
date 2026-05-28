export default {
  title: '通知中心',
  empty: '暂无通知',
  clickable: '可点击',
  refresh: '刷新',
  loading: '加载中...',
  loadMore: '加载更多',

  categories: {
    install: '安装进度',
    update: '系统更新',
    operation: '操作事件',
  },

  priority: {
    high: '高',
    medium: '中',
    low: '低',
  },

  filters: {
    all: '全部',
    searchPlaceholder: '搜索通知...',
  },

  sort: {
    newest: '最新',
    oldest: '最早',
  },

  actions: {
    markRead: '标记已读',
    markUnread: '标记未读',
    markAllRead: '全部已读',
    delete: '删除',
    deleteAll: '删除全部',
    deleteReadBefore: '清理已读',
    deleteReadBeforeTip: '清理所有已读通知',
    deleteAllConfirm: '确定要删除全部 {count} 条通知吗？此操作无法撤销。',
  },

  time: {
    justNow: '刚刚',
    minsAgo: '{n} 分钟前',
    hoursAgo: '{n} 小时前',
    daysAgo: '{n} 天前',
  },

  stats: {
    total: '共 {n} 条',
    unread: '{n} 条未读',
  },

  // ── 通知消息模板（通过 notif_key 解析） ──
  messages: {
    toolchain_installed: {
      title: '工具链已安装',
      body: '{channel} 安装成功。',
    },
    toolchain_uninstalled: {
      title: '工具链已卸载',
      body: '{name} 已移除。',
    },
    toolchain_install_failed: {
      title: '安装失败',
      body: '未能安装 {channel}: {error}',
    },
    default_toolchain_changed: {
      title: '默认工具链已更改',
      body: '默认工具链现在是 {name}。',
    },
    rust_env_installed: {
      title: 'Rust 环境已安装',
      body: 'rustup 安装成功，欢迎使用 RustVerse！',
    },
    rust_env_install_failed: {
      title: '安装失败',
      body: 'Rustup 安装失败: {error}',
    },
    crm_installed: {
      title: 'CRM 已安装',
      body: '镜像管理器 (crm) 已安装。',
    },
    plugin_installed: {
      title: '插件已安装',
      body: '{name} 已安装。',
    },
    plugin_install_failed: {
      title: '插件安装失败',
      body: '未能安装 {name}: {error}',
    },
    plugin_uninstalled: {
      title: '插件已卸载',
      body: '{name} 已卸载。',
    },
    toolchain_updates_available: {
      title: '工具链有可用更新',
      body: '{count} 个更新可用: {names}',
    },
    toolchains_updated: {
      title: '工具链已更新',
      body: '所有工具链已更新到最新版本。',
    },
    toolchain_update_failed: {
      title: '更新失败',
      body: '未能更新工具链: {error}',
    },
    rustup_updated: {
      title: 'rustup 已更新',
      body: 'rustup 已更新到最新版本。',
    },
    rustup_update_failed: {
      title: '更新失败',
      body: '未能更新 rustup: {error}',
    },
    network_diag_failed: {
      title: '网络诊断失败',
      body: '无法连接更新服务器，请检查网络连接。',
    },
    release_synced: {
      title: '发行版数据已同步',
      body: '已同步 {count} 个 {channel} 发行版。',
    },
    component_added: {
      title: '组件已添加',
      body: '{name} 已添加到 {toolchain}。',
    },
    component_removed: {
      title: '组件已移除',
      body: '{name} 已从 {toolchain} 中移除。',
    },
    target_added: {
      title: '目标平台已添加',
      body: '{name} 已添加到 {toolchain}。',
    },
    target_removed: {
      title: '目标平台已移除',
      body: '{name} 已从 {toolchain} 中移除。',
    },
    mirror_switched: {
      title: '镜像已切换',
      body: '已切换到镜像: {name}。',
    },
    mirror_best: {
      title: '最佳镜像已选择',
      body: '已选择最佳镜像，模式: {mode}。',
    },
    mirror_reset: {
      title: '镜像已重置',
      body: '已恢复至默认官方仓库。',
    },
    env_var_set: {
      title: '环境变量已设置',
      body: '{name}={value}',
    },
    env_var_removed: {
      title: '环境变量已移除',
      body: '{name} 已取消设置。',
    },
    env_var_persisted: {
      title: '环境变量已持久化',
      body: '{name}={value}',
    },
    persist_var_removed: {
      title: '持久化变量已移除',
      body: '{name} 已从系统移除。',
    },
    override_set: {
      title: '工具链重写已设置',
      body: '工具链重写: {toolchain} → {path}',
    },
    override_removed: {
      title: '工具链重写已移除',
      body: '已移除 {path} 的重写设置。',
    },
    env_check_failed: {
      title: '环境检查失败',
      body: 'rustup: {rustup}, cargo: {cargo}',
    },
  },
}
