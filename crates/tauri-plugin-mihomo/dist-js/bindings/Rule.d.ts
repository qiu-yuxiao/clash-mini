import type { RuleType } from "./RuleType";
export type Rule = {
    type: RuleType;
    index: number;
    payload: string;
    proxy: string;
    size: number;
};
