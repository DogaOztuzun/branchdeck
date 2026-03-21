import { Toast, toaster } from '@kobalte/core/toast';

export type ToastVariant = 'info' | 'success' | 'error';

const variantStyles: Record<ToastVariant, string> = {
  info: 'border-accent-info/40',
  success: 'border-accent-success/40',
  error: 'border-accent-error/40',
};

const variantTextColor: Record<ToastVariant, string> = {
  info: 'text-accent-info',
  success: 'text-accent-success',
  error: 'text-accent-error',
};

export function showToast(message: string, variant: ToastVariant = 'info') {
  toaster.show((props) => (
    <Toast
      toastId={props.toastId}
      class={`px-3 py-2 border shadow-lg bg-bg-sidebar relative ${variantStyles[variant]}`}
    >
      <Toast.Description class={`text-xs pr-4 ${variantTextColor[variant]}`}>
        {message}
      </Toast.Description>
      <Toast.CloseButton class="absolute top-1 right-1.5 text-text-dim hover:text-text-main cursor-pointer text-[10px]">
        ✕
      </Toast.CloseButton>
    </Toast>
  ));
}
