import { createSignal } from 'solid-js';
import type { CommandItem, ShortcutAction, ViewName } from '../../types/keyboard';

const [isInputFocused, setIsInputFocused] = createSignal(false);
const [isPaletteOpen, setIsPaletteOpen] = createSignal(false);
const [isOverlayOpen, setIsOverlayOpen] = createSignal(false);

const shortcuts: ShortcutAction[] = [];

function registerShortcut(action: ShortcutAction) {
  const existing = shortcuts.findIndex((s) => s.key === action.key && s.context === action.context);
  if (existing >= 0) {
    shortcuts[existing] = action;
  } else {
    shortcuts.push(action);
  }
}

function unregisterShortcut(key: string, context: ViewName | 'global') {
  const idx = shortcuts.findIndex((s) => s.key === key && s.context === context);
  if (idx >= 0) shortcuts.splice(idx, 1);
}

function getShortcutsForView(view: ViewName): ShortcutAction[] {
  return shortcuts.filter((s) => s.context === view || s.context === 'global');
}

function getCommands(): CommandItem[] {
  return shortcuts.map((s) => ({
    label: s.label,
    category: s.category,
    action: s.handler,
    shortcut: s.key,
  }));
}

function dispatch(key: string, activeView: ViewName, meta: boolean, ctrl: boolean) {
  // Cmd+K / Ctrl+K always works — toggle palette
  if (key === 'k' && (meta || ctrl)) {
    setIsPaletteOpen((v) => !v);
    return true;
  }

  // Escape: close overlays first, then propagate
  if (key === 'Escape') {
    if (isPaletteOpen()) {
      setIsPaletteOpen(false);
      return true;
    }
    if (isOverlayOpen()) {
      setIsOverlayOpen(false);
      return true;
    }
    // Let view-specific escape handlers run
    const match = shortcuts.find(
      (s) => s.key === 'Escape' && (s.context === activeView || s.context === 'global'),
    );
    if (match) {
      match.handler();
      return true;
    }
    return false;
  }

  // When palette or overlay is open, block other shortcuts
  if (isPaletteOpen() || isOverlayOpen()) return false;

  // ? toggles shortcut overlay
  if (key === '?' || (key === '/' && !isInputFocused())) {
    if (key === '?') {
      setIsOverlayOpen((v) => !v);
      return true;
    }
  }

  // When input is focused, only allow Escape and Cmd+K (handled above)
  if (isInputFocused()) return false;

  // Find matching shortcut: view-specific first, then global
  const viewMatch = shortcuts.find((s) => s.key === key && s.context === activeView);
  if (viewMatch) {
    viewMatch.handler();
    return true;
  }

  const globalMatch = shortcuts.find((s) => s.key === key && s.context === 'global');
  if (globalMatch) {
    globalMatch.handler();
    return true;
  }

  return false;
}

/** Track focus on inputs/textareas to disable single-key shortcuts */
function initFocusTracking() {
  document.addEventListener(
    'focusin',
    (e) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target as HTMLElement)?.isContentEditable) {
        setIsInputFocused(true);
      }
    },
    true,
  );
  document.addEventListener(
    'focusout',
    (e) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target as HTMLElement)?.isContentEditable) {
        setIsInputFocused(false);
      }
    },
    true,
  );
}

export function getKeyboardStore() {
  return {
    isInputFocused,
    isPaletteOpen,
    setIsPaletteOpen,
    isOverlayOpen,
    setIsOverlayOpen,
    registerShortcut,
    unregisterShortcut,
    getShortcutsForView,
    getCommands,
    dispatch,
    initFocusTracking,
  };
}
