import {Api} from "./api.js";

export const api = new Api(apiUrl());
export var currentUser = null;
export var currentConnections = [];
export var cameFromBot = null;

async function authCurrent() {
  try {
    return await api.authCurrent();
  } catch (e) {
    return null;
  }
}

async function connectionTypes() {
  try {
    return await api.connectionTypes();
  } catch (e) {
    return [];
  }
}

export async function updateGlobals() {
  let [user, connections] = await Promise.all([authCurrent(), connectionTypes()]);
  currentUser = user;
  currentConnections = connections;

  if (document.referrer.startsWith("http://localhost")) {
    cameFromBot = document.referrer;
  }
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