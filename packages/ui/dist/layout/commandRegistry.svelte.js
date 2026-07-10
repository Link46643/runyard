class CommandRegistry {
    commands = $state([]);
    register(command) {
        // Deduplicate by id
        const existing = this.commands.findIndex((c) => c.id === command.id);
        if (existing >= 0) {
            this.commands[existing] = command;
        }
        else {
            this.commands.push(command);
        }
    }
    registerMany(commands) {
        for (const cmd of commands) {
            this.register(cmd);
        }
    }
    unregister(id) {
        this.commands = this.commands.filter((c) => c.id !== id);
    }
    execute(id) {
        const cmd = this.commands.find((c) => c.id === id);
        if (cmd) {
            cmd.handler();
        }
    }
    search(query) {
        if (!query.trim())
            return this.commands;
        const q = query.toLowerCase();
        return this.commands.filter((c) => c.title.toLowerCase().includes(q) ||
            c.category.toLowerCase().includes(q) ||
            c.subtitle?.toLowerCase().includes(q));
    }
}
export const commandRegistry = new CommandRegistry();
