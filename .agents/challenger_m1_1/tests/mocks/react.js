// React Mock for testing hooks
let hookStates = []
let hookIndex = 0
let effectQueue = []
let renderCallback = null

export function resetReactMock() {
  hookStates = []
  hookIndex = 0
  effectQueue = []
}

export function setRenderCallback(cb) {
  renderCallback = cb
}

export function useState(initialVal) {
  const index = hookIndex++
  if (hookStates[index] === undefined) {
    hookStates[index] = {
      value: typeof initialVal === 'function' ? initialVal() : initialVal,
    }
  }
  const stateObj = hookStates[index]
  const setState = (newVal) => {
    const nextVal =
      typeof newVal === 'function' ? newVal(stateObj.value) : newVal
    if (stateObj.value !== nextVal) {
      stateObj.value = nextVal
      if (renderCallback) {
        renderCallback()
      }
    }
  }
  return [stateObj.value, setState]
}

export function useEffect(effect, deps) {
  const index = hookIndex++
  const oldHook = hookStates[index]
  const hasChanged =
    !oldHook || !deps || deps.some((d, i) => d !== oldHook.deps[i])

  if (hasChanged) {
    effectQueue.push({
      index,
      effect,
      cleanup: oldHook ? oldHook.cleanup : null,
    })
    hookStates[index] = { deps, cleanup: null }
  }
}

export function useRef(initialVal) {
  const index = hookIndex++
  if (hookStates[index] === undefined) {
    hookStates[index] = { current: initialVal }
  }
  return hookStates[index]
}

export function useCallback(cb, deps) {
  const index = hookIndex++
  const oldHook = hookStates[index]
  const hasChanged =
    !oldHook || !deps || deps.some((d, i) => d !== oldHook.deps[i])
  if (hasChanged) {
    hookStates[index] = { cb, deps }
    return cb
  }
  return oldHook.cb
}

export function runEffects() {
  const queue = [...effectQueue]
  effectQueue = []
  // Execute cleanups first
  for (const item of queue) {
    if (item.cleanup) {
      try {
        item.cleanup()
      } catch (e) {}
    }
  }
  // Execute effects and save cleanup functions
  for (const item of queue) {
    const cleanup = item.effect()
    hookStates[item.index].cleanup =
      typeof cleanup === 'function' ? cleanup : null
  }
}

export function triggerCleanup() {
  for (const item of hookStates) {
    if (item && item.cleanup) {
      try {
        item.cleanup()
      } catch (e) {}
      item.cleanup = null
    }
  }
}

export function runHook(hookFn, ...args) {
  hookIndex = 0
  return hookFn(...args)
}
