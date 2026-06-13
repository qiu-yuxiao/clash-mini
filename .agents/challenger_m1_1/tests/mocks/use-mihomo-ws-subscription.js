import { createJiti } from 'jiti';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Initialize a separate jiti or import the real hook
// Wait, we can import it using the same jiti instance, or since we are inside a mock module,
// we can import the real module directly.
// Let's import the real hook from the source file.
// Note: We need to resolve the path correctly.
import { useMihomoWsSubscription as realUseMihomoWsSubscription } from '../../../../src/hooks/use-mihomo-ws-subscription.ts';

export const subscriptionCalls = [];

export function clearSubscriptionCalls() {
  subscriptionCalls.length = 0;
}

export function useMihomoWsSubscription(options) {
  subscriptionCalls.push(options);
  return realUseMihomoWsSubscription(options);
}
