import type { ConnectionMetaData } from "./ConnectionMetaData";
export type Connection = {
    id: string;
    metadata: ConnectionMetaData;
    upload: number;
    download: number;
    start: string;
    chains: Array<string>;
    providerChains: Array<string> | null;
    rule: string;
    rulePayload: string;
};
