import "./index.scss";
import '@fortawesome/fontawesome-free-solid'
import React from "react";
import ReactDOM from "react-dom";
import { BrowserRouter as Router, Route } from "react-router-dom";
import { updateGlobals } from "./globals.js";
import Index from "./components/index.js";
import Players from "./components/players.js";
import Player from "./components/player.js";
import Connections from "./components/connections.js";
import Privacy from "./components/privacy.js";

function AppRouter() {
  return (
    <Router>
      <Route path="/" exact component={Index} />
      <Route path="/players" exact component={Players} />
      <Route path="/player/:id" exact component={Player} />
      <Route path="/connections" exact component={Connections} />
      <Route path="/privacy" exact component={Privacy} />
    </Router>
  );
}

updateGlobals().then(
  () => ReactDOM.render(<AppRouter />, document.getElementById("index"))
)