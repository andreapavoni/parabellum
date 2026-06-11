import { render } from "preact";
import { QueryClientProvider } from "@tanstack/preact-query";
import { App } from "./app/App";
import { AppStoreProvider } from "./state/appStore";
import { queryClient } from "./query/client";
import "./css/tailwind.css";
import "./css/style.css";

render(
  <QueryClientProvider client={queryClient}>
    <AppStoreProvider>
      <App />
    </AppStoreProvider>
  </QueryClientProvider>,
  document.getElementById("app")!,
);
