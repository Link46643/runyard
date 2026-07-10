import type { Command } from "@runyard/common";

class CommandRegistry {
  commands = $state<Command[]>([]);

  register(command: Command) {
    // Deduplicate by id
    const existing = this.commands.findIndex((c) => c.id === command.id);
    if (existing >= 0) {
      this.commands[existing] = command;
    } else {
      this.commands.push(command);
    }
  }

  registerMany(commands: Command[]) {
    for (const cmd of commands) {
      this.register(cmd);
    }
  }

  unregister(id: string) {
    this.commands = this.commands.filter((c) => c.id !== id);
  }

  execute(id: string) {
    const cmd = this.commands.find((c) => c.id === id);
    if (cmd) {
      cmd.handler();
    }
  }

  search(query: string): Command[] {
    if (!query.trim()) return this.commands;
    const q = query.toLowerCase();
    return this.commands.filter(
      (c) =>
        c.title.toLowerCase().includes(q) ||
        c.category.toLowerCase().includes(q) ||
        c.subtitle?.toLowerCase().includes(q)
    );
  }
}

export const commandRegistry = new CommandRegistry();
