import { cn } from '../../lib/cn';

type ErrorStateProps = {
  message: string;
  class?: string;
};

export function ErrorState(props: ErrorStateProps) {
  return (
    <div class={cn('text-sm font-normal text-accent-error', props.class)} role="alert">
      {props.message}
    </div>
  );
}
