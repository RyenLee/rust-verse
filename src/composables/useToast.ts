import { ref } from 'vue'

export interface ToastItem {
  id: number
  type: 'success' | 'error' | 'info'
  message: string
}

const toasts = ref<ToastItem[]>([])
let nextId = 0

function addToast(type: ToastItem['type'], message: string, duration = 3000) {
  const id = nextId++
  toasts.value.push({ id, type, message })
  if (duration > 0) {
    setTimeout(() => removeToast(id), duration)
  }
}

function removeToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

function success(message: string) {
  addToast('success', message)
}

function error(message: string) {
  addToast('error', message, 5000)
}

function info(message: string) {
  addToast('info', message)
}

export function useToast() {
  return {
    toasts,
    success,
    error,
    info,
    removeToast,
  }
}
