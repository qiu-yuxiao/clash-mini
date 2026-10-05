import type { Connection } from './Connection';
export type ConnectionsDelta = {
    epochId: string;
    sequenceId: number;
    downloadTotal: number;
    uploadTotal: number;
    memory: number;
    added: Array<Connection>;
    updated: Array<string | number>;
    removed: Array<string>;
};
