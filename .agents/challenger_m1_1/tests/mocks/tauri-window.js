export const windowState = {
  minimized: false,
  resizedCallbacks: [],
  focusChangedCallbacks: [],
};

export function resetWindowState() {
  windowState.minimized = false;
  windowState.resizedCallbacks = [];
  windowState.focusChangedCallbacks = [];
}

const mockWindow = {
  isMinimized: async () => windowState.minimized,
  onResized: async (cb) => {
    windowState.resizedCallbacks.push(cb);
    return () => {
      windowState.resizedCallbacks = windowState.resizedCallbacks.filter(c => c !== cb);
    };
  },
  onFocusChanged: async (cb) => {
    windowState.focusChangedCallbacks.push(cb);
    return () => {
      windowState.focusChangedCallbacks = windowState.focusChangedCallbacks.filter(c => c !== cb);
    };
  }
};

export function getCurrentWindow() {
  return mockWindow;
}
