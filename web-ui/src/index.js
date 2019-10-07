import "./index.scss";
import React from "react";
import ReactDOM from "react-dom";
import { BrowserRouter as Router, Route } from "react-router-dom";
import { updateGlobals } from "./globals.js";
import Index from "./components/Index.js";
import Players from "./components/Players.js";
import Player from "./components/Player.js";
import Connections from "./components/Connections.js";
import Help from "./components/Help.js";
import Privacy from "./components/Privacy.js";

import { library } from '@fortawesome/fontawesome-svg-core';
import { faQuestion, faGlobe, faCopy, faSignOutAlt, faEyeSlash, faEye, faShare, faHome, faMusic, faTrash, faCheck, faSync, faPlug } from '@fortawesome/free-solid-svg-icons';
library.add(faQuestion, faGlobe, faCopy, faSignOutAlt, faEyeSlash, faEye, faShare, faHome, faMusic, faTrash, faCheck, faSync, faPlug);
import { faTwitch, faYoutube, faSpotify } from '@fortawesome/free-brands-svg-icons';
library.add(faTwitch, faYoutube, faSpotify);

function AppRouter() {
  return (
    <Router>
      <Route path="/" exact component={Index} />
      <Route path="/players" exact component={Players} />
      <Route path="/player/:id" exact component={Player} />
      <Route path="/connections" exact component={Connections} />
      <Route path="/help" exact component={Help} />
      <Route path="/privacy" exact component={Privacy} />
    </Router>
  );
}

updateGlobals().then(
  () => ReactDOM.render(<AppRouter />, document.getElementById("index"))
)