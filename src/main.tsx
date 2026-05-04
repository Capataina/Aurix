import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { telemetry } from "./lib/telemetry";

import "./styles/tokens.css";
import "./styles/reset.css";
import "./styles/app.css";
import "./styles/components/topbar.css";
import "./styles/components/card.css";
import "./styles/components/chart.css";
import "./styles/components/blocks.css";
import "./styles/components/dashboard.css";
import "./styles/components/settings.css";
import "./styles/components/lp-backtest.css";

// Record app boot + install global click/error/lifecycle handlers.
// The telemetry singleton flushes to ~/Library/Logs/com.ataca.aurix/
// last-session.json, overwriting on every flush — no log accumulation.
telemetry.record("boot", {
  pathname: window.location.pathname,
  userAgent: navigator.userAgent,
});
telemetry.installGlobalHandlers();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
