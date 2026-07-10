export type Platform = "desktop" | "web" | "mobile";

class PlatformStore {
  current = $state<Platform>("desktop");
}

export const platform = new PlatformStore();
