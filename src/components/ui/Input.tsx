import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type InputProps = {
  class?: string;
} & Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'class'>;

export function Input(props: InputProps) {
  return (
    <input
      {...props}
      class={cn(
        'text-base px-3 py-1.5 bg-bg-main border border-border-subtle text-text-main',
        'placeholder:text-text-dim',
        'focus:outline-1 focus:outline-accent-primary focus:border-accent-primary',
        props.class,
      )}
    />
  );
}
