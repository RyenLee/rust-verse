import { vi } from 'vitest'

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

// Mock @tauri-apps/plugin-updater
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}))
