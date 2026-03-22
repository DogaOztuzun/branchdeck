import { X } from 'lucide-solid';
import { createSignal, For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { Preset } from '../../lib/commands/workspace';
import type { TabAgentInfo } from '../../lib/stores/agent';
import type { TabInfo } from '../../types/terminal';
import { AgentBadge } from './AgentBadge';

type TabBarProps = {
  tabs: TabInfo[];
  activeTabId: string | null;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string) => void;
  onNewShell: () => void;
  onNewClaude: () => void;
  presets: Preset[];
  onRunPreset: (preset: Preset) => void;
  onManagePresets: () => void;
  getTabAgent: (tabId: string) => TabAgentInfo | undefined;
};

export function TabBar(props: TabBarProps) {
  const [menuOpen, setMenuOpen] = createSignal(false);

  return (
    <div class="tab-bar px-0 select-none">
      <div class="flex flex-1 overflow-x-auto h-full">
        <For each={props.tabs}>
          {(tab) => {
            const isActive = () => props.activeTabId === tab.id;
            return (
              <button
                type="button"
                onClick={() => props.onSelectTab(tab.id)}
                class={cn(
                  'flex items-center gap-2 px-4 h-full text-xs font-medium border-r border-border-subtle transition-colors duration-150 cursor-pointer',
                  isActive()
                    ? 'bg-bg-main text-accent-primary border-t border-t-accent-primary'
                    : 'text-text-dim hover:text-text-main hover:bg-bg-main/50',
                )}
              >
                <span>{tab.title}</span>
                <Show when={tab.type === 'claude'}>
                  <AgentBadge agent={props.getTabAgent(tab.id)} />
                </Show>
                {/* biome-ignore lint/a11y/noStaticElementInteractions: nested close inside tab button */}
                {/* biome-ignore lint/a11y/useKeyWithClickEvents: close tab on click only */}
                <span
                  class="ml-1 opacity-40 hover:opacity-100 hover:text-accent-error cursor-pointer"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onCloseTab(tab.id);
                  }}
                >
                  <X size={12} />
                </span>
              </button>
            );
          }}
        </For>
      </div>
      <div class="relative">
        <button
          type="button"
          class="px-3 h-full text-xs text-text-dim hover:text-text-main cursor-pointer"
          onClick={() => setMenuOpen(!menuOpen())}
        >
          +
        </button>
        <Show when={menuOpen()}>
          <div class="absolute right-0 top-full z-50 bg-bg-sidebar border border-border-subtle shadow-lg min-w-40">
            <button
              type="button"
              class="block w-full text-left px-3 py-2 text-xs text-text-main hover:bg-bg-main cursor-pointer"
              onClick={() => {
                props.onNewShell();
                setMenuOpen(false);
              }}
            >
              New Terminal
              <span class="ml-2 text-text-dim">Ctrl+Shift+T</span>
            </button>
            <button
              type="button"
              class="block w-full text-left px-3 py-2 text-xs text-text-main hover:bg-bg-main cursor-pointer"
              onClick={() => {
                props.onNewClaude();
                setMenuOpen(false);
              }}
            >
              New Claude Code
              <span class="ml-2 text-text-dim">Ctrl+Shift+A</span>
            </button>
            <Show when={props.presets.length > 0}>
              <div class="border-t border-border-subtle my-1" />
              <For each={props.presets}>
                {(preset) => (
                  <button
                    type="button"
                    class="flex items-center gap-2 w-full text-left px-3 py-2 text-xs text-text-main hover:bg-bg-main cursor-pointer"
                    onClick={() => {
                      props.onRunPreset(preset);
                      setMenuOpen(false);
                    }}
                  >
                    <span>{preset.name}</span>
                    <span class="ml-auto text-text-dim text-xs">{preset.tabType}</span>
                  </button>
                )}
              </For>
            </Show>
            <div class="border-t border-border-subtle my-1" />
            <button
              type="button"
              class="block w-full text-left px-3 py-2 text-xs text-text-dim hover:text-text-main hover:bg-bg-main cursor-pointer"
              onClick={() => {
                props.onManagePresets();
                setMenuOpen(false);
              }}
            >
              Manage Presets...
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}
