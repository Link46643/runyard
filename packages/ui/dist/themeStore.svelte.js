class ThemeStore {
    current = $state("dark");
    constructor() {
        this.load();
        this.apply();
    }
    load() {
        if (typeof window !== "undefined") {
            const saved = localStorage.getItem("runyard:theme");
            if (saved) {
                this.current = saved;
            }
        }
    }
    save() {
        if (typeof window !== "undefined") {
            localStorage.setItem("runyard:theme", this.current);
        }
    }
    toggle() {
        this.current = this.current === "light" ? "dark" : "light";
        this.save();
        this.apply();
    }
    set(theme) {
        this.current = theme;
        this.save();
        this.apply();
    }
    apply() {
        if (typeof document !== "undefined") {
            document.documentElement.setAttribute("data-theme", this.current);
        }
    }
}
export const theme = new ThemeStore();
