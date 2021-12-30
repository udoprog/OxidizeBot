import "./index.scss";
import React from "react";
import ReactDOM from "react-dom";
import { BrowserRouter } from "react-router-dom";
import { updateGlobals } from "./globals.js";
import App from "./components/App.js";

import { library } from '@fortawesome/fontawesome-svg-core';
import { faQuestion, faGlobe, faCopy, faSignOutAlt, faEyeSlash, faEye, faShare, faHome, faMusic, faTrash, faCheck, faSync, faPlug } from '@fortawesome/free-solid-svg-icons';
library.add(faQuestion, faGlobe, faCopy, faSignOutAlt, faEyeSlash, faEye, faShare, faHome, faMusic, faTrash, faCheck, faSync, faPlug);
import { faTwitch, faYoutube, faSpotify, faTwitter, faGithub } from '@fortawesome/free-brands-svg-icons';
library.add(faTwitch, faYoutube, faSpotify, faTwitter, faGithub);

updateGlobals().then(
  () => ReactDOM.render(
    <React.StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </React.StrictMode>,
    document.getElementById("index")
  )
)
