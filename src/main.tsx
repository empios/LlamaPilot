import React from "react";
import ReactDOM from "react-dom/client";

import { AppShell } from "@/app/app-shell";
import { Providers } from "@/app/providers";
import "@/index.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element is missing from index.html");
}

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <Providers>
      <AppShell />
    </Providers>
  </React.StrictMode>,
);
