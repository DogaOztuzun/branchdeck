import { Toast, toaster } from '@kobalte/core/toast';
import { For } from 'solid-js';
import type { ToastVariant } from '../../lib/stores/toast';

const variantStyles: Record<ToastVariant, string> = {
  info: 'border-accent-info/40 bg-accent-info/5',
  success: 'border-accent-success/40 bg-accent-success/5',
  error: 'border-accent-error/40 bg-accent-error/5',
};

const variantTextColor: Record<ToastVariant, string> = {
  info: 'text-accent-info',
  success: 'text-accent-success',
  error: 'text-accent-error',
};

export function ToastRegion() {
  return (
    <Toast.Region duration={4000} limit={3}>
      <Toast.List class="fixed bottom-4 right-4 z-[100] flex flex-col gap-2 w-80" />
      <For each={toaster.toasts()}>
        {(toast) => {
          const data = toast.data as { message: string; variant: ToastVariant } | undefined;
          const variant = data?.variant ?? 'info';
          const message = data?.message ?? '';
          return (
            <Toast
              toastId={toast.id}
              class={`px-3 py-2 border shadow-lg ${variantStyles[variant]} bg-bg-sidebar`}
            >
              <div class="flex items-start justify-between gap-2">
                <Toast.Description class={`text-xs ${variantTextColor[variant]}`}>
                  {message}
                </Toast.Description>
                <Toast.CloseButton class="text-text-dim hover:text-text-main cursor-pointer text-xs shrink-0">
                  ✕
                </Toast.CloseButton>
              </div>
            </Toast>
          );
        }}
      </For>
    </Toast.Region>
  );
}
