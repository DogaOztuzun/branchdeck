import type { JSX } from 'solid-js';
import { Show } from 'solid-js';
import { cn } from '../../lib/cn';

type SectionHeaderProps = {
  label: string;
  action?: JSX.Element;
  class?: string;
};

export function SectionHeader(props: SectionHeaderProps) {
  return (
    <div class={cn('flex items-center justify-between px-3 mb-2 mt-4', props.class)}>
      <span class="text-[10px] font-bold uppercase tracking-widest text-text-dim">
        {props.label}
      </span>
      <Show when={props.action}>{props.action}</Show>
    </div>
  );
}
