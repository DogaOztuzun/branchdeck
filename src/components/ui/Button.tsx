import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline';
type ButtonSize = 'default' | 'compact' | 'icon';

type ButtonProps = {
  variant?: ButtonVariant;
  size?: ButtonSize;
  class?: string;
  children: JSX.Element;
} & Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, 'class'>;

const variantStyles: Record<ButtonVariant, string> = {
  primary: 'z-button-variant-default',
  secondary: 'z-button-variant-secondary',
  ghost: 'z-button-variant-ghost',
  danger: 'z-button-variant-destructive',
  outline: 'z-button-variant-outline',
};

const sizeStyles: Record<ButtonSize, string> = {
  default: 'z-button-size-default',
  compact: 'z-button-size-sm',
  icon: 'z-button-size-icon',
};

export function Button(props: ButtonProps) {
  return (
    <button
      {...props}
      type={props.type ?? 'button'}
      class={cn(
        'z-button',
        variantStyles[props.variant ?? 'secondary'],
        sizeStyles[props.size ?? 'default'],
        props.class,
      )}
    >
      {props.children}
    </button>
  );
}
