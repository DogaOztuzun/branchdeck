import type { ComponentProps } from 'solid-js';
import { splitProps } from 'solid-js';
import { cn } from '../../lib/cn';

type TextareaProps = ComponentProps<'textarea'>;

const Textarea = (props: TextareaProps) => {
  const [local, others] = splitProps(props, ['class']);
  return (
    <textarea
      data-slot="textarea"
      class={cn('z-input min-h-[60px] resize-y', local.class)}
      {...others}
    />
  );
};

export { Textarea };
