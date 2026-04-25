import { render } from "preact";
import { App } from "./app/App";
import { AppStoreProvider } from "./state/appStore";
import "./css/style.css";

render(
  <AppStoreProvider>
    <App />
  </AppStoreProvider>,
  document.getElementById("app")!,
);
