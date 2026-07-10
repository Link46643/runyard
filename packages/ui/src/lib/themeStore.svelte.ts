class ThemeStore {
  current = $state<"light" | "dark">("dark");

  constructor() {
    this.load();
    this.apply();
  }

  load() {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("runyard:theme") as "light" | "dark";
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

  set(theme: "light" | "dark") {
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
