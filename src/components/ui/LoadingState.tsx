import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type LoadingStateProps = {
  children: JSX.Element;
  class?: string;
};

export function LoadingState(props: LoadingStateProps) {
  return <div class={cn('animate-pulse-opacity', props.class)}>{props.children}</div>;
}

type LoadingTextProps = {
  class?: string;
};

export function LoadingText(props: LoadingTextProps) {
  return (
    <div class={cn('flex items-center justify-center py-4', props.class)}>
      <span class="text-[11px] font-normal text-text-dim animate-pulse-opacity">Loading...</span>
    </div>
  );
}
