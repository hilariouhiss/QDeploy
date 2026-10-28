import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

function showFatal(message: string) {
  const host = document.getElementById("app");
  if (!host) return;
  const text = (message || "未知错误").split("\n")[0].slice(0, 300);
  host.innerHTML = "";
  const box = document.createElement("div");
  box.className = "boot boot-error";
  box.innerHTML =
    '<svg class="boot-mark" viewBox="0 0 20 20" aria-hidden="true">' +
    '<rect x="0" y="0" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="7.2" y="0" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="14.4" y="0" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="0" y="7.2" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="7.2" y="7.2" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="14.4" y="7.2" width="5.6" height="5.6" rx="1" fill="#dfe3e5"/>' +
    '<rect x="0" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="7.2" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    '<rect x="14.4" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416"/>' +
    "</svg>" +
    '<div class="boot-name">QDeploy</div>' +
    '<div class="boot-note">界面出错了：' +
    text.replace(/[<>&]/g, "") +
    "<br />详细日志在 %APPDATA%/QDeploy/logs</div>";
  host.appendChild(box);
}

window.addEventListener("error", (e) => showFatal(e.message));
window.addEventListener("unhandledrejection", (e) =>
  showFatal(e.reason instanceof Error ? e.reason.message : String(e.reason)),
);

const app = createApp(App);
app.config.errorHandler = (err) => showFatal(err instanceof Error ? err.message : String(err));
app.mount("#app");
