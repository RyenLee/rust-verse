import { invoke } from '@tauri-apps/api/core'

/**
 * Bridge frontend log messages to the backend file logger.
 * All logs are written to data/logs/rustverse.log in the app directory.
 */
function sendLog(level: string, module: string, message: string) {
  invoke('frontend_log', { level, module, message }).catch(() => {
    // Silently ignore if backend is not ready (e.g., during startup)
  })
}

export const appLog = {
  debug(module: string, message: string) {
    sendLog('DEBUG', module, message)
  },
  info(module: string, message: string) {
    sendLog('INFO', module, message)
  },
  warn(module: string, message: string) {
    sendLog('WARN', module, message)
  },
  error(module: string, message: string) {
    sendLog('ERROR', module, message)
  },
}

/** Get the log directory path from the backend. */
export async function getLogDir(): Promise<string> {
  return invoke<string>('get_log_dir')
}
