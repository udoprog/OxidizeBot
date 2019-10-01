import Api from "./api.js";

export const api = new Api(apiUrl());
export var currentUser = null;
export var currentConnections = [];

export async function updateGlobals() {
  let [user, connections] = await Promise.all([api.authCurrent(), api.connectionTypes()]);
  currentUser = user;
  currentConnections = connections;
}

/**
 * Get the current URL to connect to.
 */
function apiUrl() {
  var loc = window.location;
  var scheme = "http";

  if (loc.protocol === "https:") {
    scheme = "https";
  }

  return `${scheme}://${loc.host}/api`;
}