import type { Connection } from './Connection';
export type ConnectionsSnapshot = {
    epochId: string;
    sequenceId: number;
    downloadTotal: number;
    uploadTotal: number;
    connections: Array<Connection>;
    memory: number;
};
