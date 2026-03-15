import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal } from '@xterm/xterm';
import { createEffect, onCleanup, onMount } from 'solid-js';
import { resizeTerminal, writeTerminal } from '../../lib/commands/terminal';
import { getTerminalStore } from '../../lib/stores/terminal';

type TerminalViewProps = {
  sessionId: string;
  visible: boolean;
};

export function TerminalView(props: TerminalViewProps) {
  let containerRef: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  let resizeObserver: ResizeObserver | undefined;

  onMount(() => {
    if (!containerRef) return;

    const store = getTerminalStore();

    terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      theme: {
        background: '#1a1b26',
        foreground: '#c0caf5',
        cursor: '#c0caf5',
        selectionBackground: '#414868',
        black: '#15161e',
        red: '#f7768e',
        green: '#9ece6a',
        yellow: '#e0af68',
        blue: '#7aa2f7',
        magenta: '#bb9af7',
        cyan: '#7dcfff',
        white: '#a9b1d6',
        brightBlack: '#414868',
        brightRed: '#f7768e',
        brightGreen: '#9ece6a',
        brightYellow: '#e0af68',
        brightBlue: '#7aa2f7',
        brightMagenta: '#bb9af7',
        brightCyan: '#7dcfff',
        brightWhite: '#c0caf5',
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(containerRef);

    try {
      const webglAddon = new WebglAddon();
      terminal.loadAddon(webglAddon);
    } catch {
      // WebGL not available, canvas fallback is automatic
    }

    fitAddon.fit();

    terminal.onData((data) => {
      const encoder = new TextEncoder();
      writeTerminal(props.sessionId, encoder.encode(data));
    });

    terminal.onResize(({ rows, cols }) => {
      resizeTerminal(props.sessionId, rows, cols);
    });

    store.registerOutputHandler(props.sessionId, (data: Uint8Array) => {
      terminal?.write(data);
    });

    resizeObserver = new ResizeObserver(() => {
      if (props.visible) {
        fitAddon?.fit();
      }
    });
    resizeObserver.observe(containerRef);
  });

  createEffect(() => {
    if (props.visible && fitAddon && terminal) {
      requestAnimationFrame(() => {
        fitAddon?.fit();
        terminal?.focus();
      });
    }
  });

  onCleanup(() => {
    const store = getTerminalStore();
    store.unregisterOutputHandler(props.sessionId);
    resizeObserver?.disconnect();
    terminal?.dispose();
  });

  return (
    <div
      ref={containerRef}
      class="h-full w-full"
      style={{ display: props.visible ? 'block' : 'none' }}
    />
  );
}
