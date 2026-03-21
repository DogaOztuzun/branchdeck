import { Toast } from '@kobalte/core/toast';

export function ToastRegion() {
  return (
    <Toast.Region duration={4000} limit={3}>
      <Toast.List class="fixed bottom-4 right-4 z-[100] flex flex-col gap-2 w-80" />
    </Toast.Region>
  );
}
