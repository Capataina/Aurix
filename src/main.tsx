import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";

import "./styles/tokens.css";
import "./styles/reset.css";
import "./styles/app.css";
import "./styles/components/topbar.css";
import "./styles/components/card.css";
import "./styles/components/chart.css";
import "./styles/components/blocks.css";
import "./styles/components/dashboard.css";
import "./styles/components/lp-backtest.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
