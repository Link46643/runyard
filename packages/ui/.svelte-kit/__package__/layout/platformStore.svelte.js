class PlatformStore {
    current = $state("desktop");
    constructor() {
        if (typeof window !== "undefined") {
            const updatePlatform = () => {
                if (window.innerWidth < 768) {
                    this.current = "mobile";
                }
                else if (window.__TAURI_INTERNALS__) {
                    this.current = "desktop";
                }
                else {
                    this.current = "web";
                }
            };
            updatePlatform();
            window.addEventListener("resize", updatePlatform);
        }
    }
}
export const platform = new PlatformStore();
