import { toaster } from '@kobalte/core/toast';

export type ToastVariant = 'info' | 'success' | 'error';

export function showToast(message: string, variant: ToastVariant = 'info') {
  toaster.show((props) => ({
    ...props,
    message,
    variant,
  }));
}
