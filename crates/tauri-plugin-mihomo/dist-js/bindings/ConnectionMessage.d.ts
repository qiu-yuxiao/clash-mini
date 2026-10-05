import type { ConnectionsDelta } from './ConnectionsDelta';
import type { ConnectionsSnapshot } from './ConnectionsSnapshot';
export type ConnectionMessage = {
    type: 'snapshot';
    data: ConnectionsSnapshot;
} | {
    type: 'delta';
    data: ConnectionsDelta;
};
