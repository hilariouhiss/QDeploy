import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

const MARK =
  '<svg class="boot-mark" viewBox="0 0 18 18" aria-hidden="true">' +
  '<rect x="0" y="0" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="6.6" y="0" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="13.2" y="0" width="4.8" height="4.8" rx="0.9" fill="#b9bec5"/>' +
  '<rect x="0" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="6.6" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="13.2" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="0" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="6.6" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  '<rect x="13.2" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a"/>' +
  "</svg>";

function showFatal(message: string) {
  const host = document.getElementById("app");
  if (!host) return;
  const text = (message || "未知错误").split("\n")[0].slice(0, 300);
  host.innerHTML = "";
  const box = document.createElement("div");
  box.className = "boot boot-error";
  box.innerHTML =
    MARK +
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
