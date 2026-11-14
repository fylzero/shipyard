import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-700.css";
import "./styles.css";
import "./theme";

createApp(App).use(router).mount("#app");
