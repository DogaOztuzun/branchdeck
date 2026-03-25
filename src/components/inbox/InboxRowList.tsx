import { For } from 'solid-js';
import { getInboxStore } from '../../lib/stores/inbox';
import { SectionHeader } from '../ui/SectionHeader';
import { InboxRow } from './InboxRow';

export function InboxRowList() {
  const inbox = getInboxStore();

  const flat = () => inbox.flatItems();

  return (
    <div>
      <For each={inbox.groups()}>
        {(group) => (
          <div>
            <SectionHeader
              label={group.label}
              count={group.items.length}
              class={
                group.color === 'error'
                  ? 'text-accent-error'
                  : group.color === 'success'
                    ? 'text-accent-success'
                    : 'text-text-dim'
              }
            />
            <For each={group.items}>
              {(item) => {
                const flatIdx = () => flat().findIndex((i) => i.id === item.id);

                const handleClick = () => {
                  const idx = flatIdx();
                  if (inbox.selectedIndex() === idx && inbox.expandedId() === item.id) {
                    inbox.setExpandedId(null);
                  } else {
                    inbox.setSelectedIndex(idx);
                    inbox.setExpandedId(item.id);
                  }
                };

                return (
                  <InboxRow
                    item={item}
                    selected={inbox.selectedIndex() === flatIdx()}
                    expanded={inbox.expandedId() === item.id}
                    showRepo={inbox.isMultiRepo()}
                    onClick={handleClick}
                    onMerge={() => inbox.mergeSelected()}
                    onDismiss={() => inbox.dismissSelected()}
                  />
                );
              }}
            </For>
          </div>
        )}
      </For>
    </div>
  );
}
