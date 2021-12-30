import React from "react";
import { Routes, Route, useLocation, useSearchParams } from "react-router-dom";
import Layout from "./Layout.js";
import Index from "./Index.js";
import Playlists from "./Playlists.js";
import Player from "./Player.js";
import Connections from "./Connections.js";
import Help from "./Help.js";
import Privacy from "./Privacy.js";

function ConnectionsWithRoute() {
  let location = useLocation();
  return <Connections location={location} />;
}

function HelpWithRoute() {
  let [searchParams, setSearchParams] = useSearchParams();
  return <Help searchParams={searchParams} setSearchParams={setSearchParams} />;
}

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route path="/" element={<Index />} />
        <Route path="/playlists" element={<Playlists />} />
        <Route path="/player/:id" element={<Player />} />
        <Route path="/connections" element={<ConnectionsWithRoute />} />
        <Route path="/help" element={<HelpWithRoute />} />
        <Route path="/privacy" element={<Privacy />} />
      </Route>
    </Routes>
  );
}
