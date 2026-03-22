import { Dialog } from '@kobalte/core';
import { createEffect, createSignal, For, Show } from 'solid-js';
import type { Preset } from '../../lib/commands/workspace';
import { getPresets, savePresets } from '../../lib/commands/workspace';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/Select';

type PresetManagerProps = {
  open: boolean;
  repoPath: string;
  onClose: () => void;
  onPresetsChanged: () => void;
};

type EditingPreset = {
  name: string;
  command: string;
  tabType: 'shell' | 'claude';
};

export function PresetManager(props: PresetManagerProps) {
  const [presets, setPresets] = createSignal<Preset[]>([]);
  const [adding, setAdding] = createSignal(false);
  const [editingIndex, setEditingIndex] = createSignal<number | null>(null);
  const [form, setForm] = createSignal<EditingPreset>({ name: '', command: '', tabType: 'shell' });
  const [error, setError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  createEffect(() => {
    if (props.open) {
      setAdding(false);
      setEditingIndex(null);
      setError(null);
      setForm({ name: '', command: '', tabType: 'shell' });
      getPresets(props.repoPath)
        .then((result) => setPresets(result))
        .catch((e) => setError(String(e)));
    }
  });

  function resetForm() {
    setForm({ name: '', command: '', tabType: 'shell' });
    setAdding(false);
    setEditingIndex(null);
  }

  async function handleSaveNew() {
    const f = form();
    if (!f.name.trim()) return;

    setSaving(true);
    setError(null);
    try {
      const newPreset: Preset = {
        name: f.name.trim(),
        command: f.command,
        tabType: f.tabType,
        env: {},
      };
      const updated = [...presets(), newPreset];
      await savePresets(props.repoPath, updated);
      setPresets(updated);
      resetForm();
      props.onPresetsChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleSaveEdit(index: number) {
    const f = form();
    if (!f.name.trim()) return;

    setSaving(true);
    setError(null);
    try {
      const updated = [...presets()];
      updated[index] = {
        ...updated[index],
        name: f.name.trim(),
        command: f.command,
        tabType: f.tabType,
      };
      await savePresets(props.repoPath, updated);
      setPresets(updated);
      resetForm();
      props.onPresetsChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(index: number) {
    setSaving(true);
    setError(null);
    try {
      const updated = presets().filter((_, i) => i !== index);
      await savePresets(props.repoPath, updated);
      setPresets(updated);
      resetForm();
      props.onPresetsChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function startEdit(index: number) {
    const p = presets()[index];
    setForm({ name: p.name, command: p.command, tabType: p.tabType });
    setEditingIndex(index);
    setAdding(false);
  }

  function startAdd() {
    setForm({ name: '', command: '', tabType: 'shell' });
    setAdding(true);
    setEditingIndex(null);
  }

  return (
    <Dialog.Root
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-40 bg-black/50" />
        <Dialog.Content class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 bg-bg-sidebar border border-border-subtle shadow-lg p-4 max-h-[80vh] overflow-y-auto">
          <Dialog.Title class="text-sm font-semibold text-text-main mb-3">
            Manage Presets
          </Dialog.Title>

          <div class="space-y-0">
            <For each={presets()}>
              {(preset, i) => (
                <Show
                  when={editingIndex() === i()}
                  fallback={
                    <div class="flex items-center justify-between px-3 py-2 text-xs border-b border-border-subtle">
                      <div>
                        <span class="text-text-main">{preset.name}</span>
                        <span class="ml-2 text-text-dim text-[10px]">{preset.tabType}</span>
                      </div>
                      <div class="flex gap-2">
                        <button
                          type="button"
                          class="text-text-dim hover:text-text-main cursor-pointer"
                          onClick={() => startEdit(i())}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          class="text-text-dim hover:text-accent-error cursor-pointer"
                          disabled={saving()}
                          onClick={() => handleDelete(i())}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  }
                >
                  <div class="px-3 py-2 border-b border-border-subtle space-y-2">
                    <input
                      type="text"
                      placeholder="Preset name"
                      value={form().name}
                      onInput={(e) => setForm((f) => ({ ...f, name: e.currentTarget.value }))}
                      class="w-full bg-bg-main border border-border-subtle text-text-main text-xs px-3 py-1.5 focus:outline-none focus:border-accent-primary"
                    />
                    <input
                      type="text"
                      placeholder="Command"
                      value={form().command}
                      onInput={(e) => setForm((f) => ({ ...f, command: e.currentTarget.value }))}
                      class="w-full bg-bg-main border border-border-subtle text-text-main text-xs px-3 py-1.5 focus:outline-none focus:border-accent-primary"
                    />
                    <Select
                      options={['shell', 'claude']}
                      value={form().tabType}
                      onChange={(val) =>
                        setForm((f) => ({
                          ...f,
                          tabType: (val ?? 'shell') as 'shell' | 'claude',
                        }))
                      }
                      placeholder="Tab type"
                      itemComponent={(props) => (
                        <SelectItem item={props.item}>
                          {props.item.rawValue === 'shell' ? 'Shell' : 'Claude'}
                        </SelectItem>
                      )}
                    >
                      <SelectTrigger>
                        <SelectValue<string>>
                          {(state) => (state.selectedOption() === 'shell' ? 'Shell' : 'Claude')}
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent />
                    </Select>
                    <div class="flex justify-end gap-2">
                      <button
                        type="button"
                        class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50"
                        onClick={() => resetForm()}
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        disabled={saving() || !form().name.trim()}
                        class="px-3 py-1.5 text-xs bg-accent-primary text-bg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90"
                        onClick={() => handleSaveEdit(i())}
                      >
                        {saving() ? 'Saving...' : 'Save'}
                      </button>
                    </div>
                  </div>
                </Show>
              )}
            </For>
          </div>

          <Show when={presets().length === 0 && !adding()}>
            <p class="text-xs text-text-dim py-4 text-center">No presets yet</p>
          </Show>

          <Show when={error()}>
            <p class="mt-2 text-xs text-accent-error">{error()}</p>
          </Show>

          <Show
            when={adding()}
            fallback={
              <button
                type="button"
                class="mt-3 w-full px-3 py-1.5 text-xs border border-border-subtle text-text-dim hover:text-text-main hover:border-accent-primary cursor-pointer"
                onClick={() => startAdd()}
              >
                + Add Preset
              </button>
            }
          >
            <div class="mt-3 space-y-2">
              <input
                type="text"
                placeholder="Preset name"
                value={form().name}
                onInput={(e) => setForm((f) => ({ ...f, name: e.currentTarget.value }))}
                autofocus
                class="w-full bg-bg-main border border-border-subtle text-text-main text-xs px-3 py-1.5 focus:outline-none focus:border-accent-primary"
              />
              <input
                type="text"
                placeholder="Command"
                value={form().command}
                onInput={(e) => setForm((f) => ({ ...f, command: e.currentTarget.value }))}
                class="w-full bg-bg-main border border-border-subtle text-text-main text-xs px-3 py-1.5 focus:outline-none focus:border-accent-primary"
              />
              <Select
                options={['shell', 'claude']}
                value={form().tabType}
                onChange={(val) =>
                  setForm((f) => ({
                    ...f,
                    tabType: (val ?? 'shell') as 'shell' | 'claude',
                  }))
                }
                placeholder="Tab type"
                itemComponent={(props) => (
                  <SelectItem item={props.item}>
                    {props.item.rawValue === 'shell' ? 'Shell' : 'Claude'}
                  </SelectItem>
                )}
              >
                <SelectTrigger>
                  <SelectValue<string>>
                    {(state) => (state.selectedOption() === 'shell' ? 'Shell' : 'Claude')}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
              <div class="flex justify-end gap-2">
                <button
                  type="button"
                  class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50"
                  onClick={() => resetForm()}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  disabled={saving() || !form().name.trim()}
                  class="px-3 py-1.5 text-xs bg-accent-primary text-bg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90"
                  onClick={() => handleSaveNew()}
                >
                  {saving() ? 'Saving...' : 'Add Preset'}
                </button>
              </div>
            </div>
          </Show>

          <div class="mt-4 flex justify-end">
            <button
              type="button"
              class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50"
              onClick={() => props.onClose()}
            >
              Close
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
