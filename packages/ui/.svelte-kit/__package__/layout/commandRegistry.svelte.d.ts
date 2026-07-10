import type { Command } from "@runyard/common";
declare class CommandRegistry {
    commands: Command[];
    register(command: Command): void;
    registerMany(commands: Command[]): void;
    unregister(id: string): void;
    execute(id: string): void;
    search(query: string): Command[];
}
export declare const commandRegistry: CommandRegistry;
export {};
