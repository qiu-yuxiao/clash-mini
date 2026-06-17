import { createJiti } from 'jiti'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Create jiti with aliases pointing to mocks
const jiti = createJiti(import.meta.url, {
  alias: {
    react: path.resolve(__dirname, 'mocks/react.js'),
    '@tauri-apps/api/window': path.resolve(__dirname, 'mocks/tauri-window.js'),
    'tauri-plugin-mihomo-api': path.resolve(
      __dirname,
      'mocks/tauri-plugin-mihomo-api.js',
    ),
    '@tanstack/react-query': path.resolve(
      __dirname,
      'mocks/tanstack-react-query.js',
    ),
    'foxact/use-local-storage': path.resolve(
      __dirname,
      'mocks/foxact-use-local-storage.js',
    ),
    '@/services/cmds': path.resolve(__dirname, 'mocks/services-cmds.js'),
    './use-clash-log': path.resolve(__dirname, 'mocks/use-clash-log.js'),
    './use-traffic-monitor': path.resolve(
      __dirname,
      'mocks/use-traffic-monitor.js',
    ),
    './use-mihomo-ws-subscription': path.resolve(
      __dirname,
      'mocks/use-mihomo-ws-subscription.js',
    ),
  },
})

// Import mock modules for controlling their states
const { resetReactMock, setRenderCallback, runEffects, runHook } =
  await jiti.import('./mocks/react.js')
const { windowState, resetWindowState } = await jiti.import(
  './mocks/tauri-window.js',
)
const { queryClient } = await jiti.import('./mocks/tanstack-react-query.js')
const { subscriptionCalls, clearSubscriptionCalls } = await jiti.import(
  './mocks/use-mihomo-ws-subscription.js',
)

// Import the real hooks under test
const { useVisibility } = await jiti.import(
  '../../../src/hooks/use-visibility.ts',
)
const { useTrafficData } = await jiti.import(
  '../../../src/hooks/use-traffic-data.ts',
)
const { useLogData } = await jiti.import('../../../src/hooks/use-log-data.ts')

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

// Helper to test a hook
function testHook(hookFn, ...args) {
  const result = { current: null }
  const render = () => {
    result.current = runHook(hookFn, ...args)
  }
  setRenderCallback(render)
  resetReactMock()
  render()
  runEffects()
  return {
    result,
    rerender: () => {
      render()
      runEffects()
    },
  }
}

// Global Document Mock Setup
const documentListeners = {}
globalThis.document = {
  visibilityState: 'visible',
  addEventListener: (event, cb) => {
    if (!documentListeners[event]) documentListeners[event] = []
    documentListeners[event].push(cb)
  },
  removeEventListener: (event, cb) => {
    if (documentListeners[event]) {
      documentListeners[event] = documentListeners[event].filter(
        (c) => c !== cb,
      )
    }
  },
}

function triggerDocumentEvent(event) {
  if (documentListeners[event]) {
    for (const cb of documentListeners[event]) {
      cb()
    }
  }
}

async function runTests() {
  console.log('--- STARTING VERIFICATION TESTS ---')
  let failures = 0

  function assert(condition, message) {
    if (condition) {
      console.log(`[PASS] ${message}`)
    } else {
      console.error(`[FAIL] ${message}`)
      failures++
    }
  }

  // ==========================================
  // Test 1: useVisibility Transitions
  // ==========================================
  console.log('\n--- Test Case 1: useVisibility Transitions ---')

  // Setup initial visible state
  globalThis.document.visibilityState = 'visible'
  resetWindowState()

  const { result: visResult, rerender: rerenderVis } = testHook(useVisibility)

  // Wait for initTauri inside useEffect to resolve
  await sleep(10)
  runEffects()

  assert(
    visResult.current === true,
    'Initially, document visible and not minimized -> useVisibility is true',
  )

  // Change document visibility to hidden
  globalThis.document.visibilityState = 'hidden'
  triggerDocumentEvent('visibilitychange')
  runEffects()
  assert(
    visResult.current === false,
    'Document visibility becomes hidden -> useVisibility is false',
  )

  // Change document visibility back to visible
  globalThis.document.visibilityState = 'visible'
  triggerDocumentEvent('visibilitychange')
  runEffects()
  assert(
    visResult.current === true,
    'Document visibility becomes visible -> useVisibility is true',
  )

  // Simulate window minimize
  windowState.minimized = true
  for (const cb of windowState.resizedCallbacks) {
    await cb()
  }
  // Allow async isMinimized check to settle
  await sleep(10)
  runEffects()
  assert(
    visResult.current === false,
    'Window minimized -> useVisibility is false',
  )

  // Simulate window restore
  windowState.minimized = false
  for (const cb of windowState.resizedCallbacks) {
    await cb()
  }
  // Allow async isMinimized check to settle
  await sleep(10)
  runEffects()
  assert(
    visResult.current === true,
    'Window restored (not minimized) -> useVisibility is true',
  )

  // ==========================================
  // Test 2: useTrafficData Key Nullification
  // ==========================================
  console.log(
    '\n--- Test Case 2: useTrafficData Subscription Key Nullification ---',
  )

  // Reset to visible state
  globalThis.document.visibilityState = 'visible'
  resetWindowState()
  clearSubscriptionCalls()

  const { result: trafficResult } = testHook(useTrafficData)
  await sleep(10)
  runEffects()

  assert(
    subscriptionCalls.length > 0,
    'useTrafficData registers a subscription call',
  )
  let latestCall = subscriptionCalls[subscriptionCalls.length - 1]
  let key = latestCall.buildSubscriptKey(12345)
  assert(
    key === 'getClashTraffic-12345',
    `Subscription key initially active: "${key}"`,
  )

  // Now change document visibility to hidden
  globalThis.document.visibilityState = 'hidden'
  triggerDocumentEvent('visibilitychange')
  runEffects()

  latestCall = subscriptionCalls[subscriptionCalls.length - 1]
  key = latestCall.buildSubscriptKey(12345)
  assert(
    key === null,
    `Subscription key correctly set to null when visibility is false`,
  )

  // ==========================================
  // Test 3: useLogData Key Nullification
  // ==========================================
  console.log(
    '\n--- Test Case 3: useLogData Subscription Key Nullification ---',
  )

  // Reset to visible state
  globalThis.document.visibilityState = 'visible'
  resetWindowState()
  clearSubscriptionCalls()

  const { result: logResult } = testHook(useLogData)
  await sleep(10)
  runEffects()

  assert(
    subscriptionCalls.length > 0,
    'useLogData registers a subscription call',
  )
  latestCall = subscriptionCalls[subscriptionCalls.length - 1]
  key = latestCall.buildSubscriptKey(12345)
  assert(
    key === 'getClashLog-12345',
    `Subscription key initially active: "${key}"`,
  )

  // Now change document visibility to hidden
  globalThis.document.visibilityState = 'hidden'
  triggerDocumentEvent('visibilitychange')
  runEffects()

  latestCall = subscriptionCalls[subscriptionCalls.length - 1]
  key = latestCall.buildSubscriptKey(12345)
  assert(
    key === null,
    `Subscription key correctly set to null when visibility is false`,
  )

  console.log('\n--- VERIFICATION COMPLETE ---')
  if (failures > 0) {
    console.error(`Result: FAILED (${failures} assertions failed)`)
    process.exit(1)
  } else {
    console.log('Result: ALL PASSED')
    process.exit(0)
  }
}

runTests().catch((err) => {
  console.error('Test run failed:', err)
  process.exit(1)
})
