export default class Api {
  constructor(url) {
    this.url = url;
  }

  /**
   *
   * @param {string | array<string>} path
   * @param {*} data
   */
  fetch(path, data = {}) {
    if (path instanceof Array) {
      path = encodePath(path);
    }

    data.credentials = "same-origin";

    return fetch(`${this.url}/${path}`, data).then((r) => {
      if (!r.ok) {
        return r.text().then(text => {
          throw Error(`got bad status code: ${r.status}: ${text}`);
        });
      }

      return r.json();
    });
  }

  /**
   * Get list of players.
   */
  players() {
    return this.fetch(["players"]);
  }

  /**
   * Get information about the specified player.
   */
  player(id) {
    return this.fetch(["player", id]);
  }

  /**
   * Login the current user.
   */
  authLogin() {
    return this.fetch(["auth", "login"], {method: "POST"});
  }

  /**
   * Logout the current user.
   */
  authLogout() {
    return this.fetch(["auth", "logout"], {method: "POST"});
  }

  /**
   * Get information on the current user.
   */
  authCurrent() {
    return this.fetch(["auth", "current"]);
  }

  /**
   * List all available connections.
   */
  connectionsList() {
    return this.fetch(["connections"]);
  }

  /**
   * Remove the given connection.
   */
  connectionsRemove(id) {
    return this.fetch(["connections", id], {method: "DELETE"});
  }

  /**
   * Prepare to create the given connection.
   */
  connectionsCreate(id) {
    return this.fetch(["connections", id], {method: "POST"});
  }

  /**
   * Get a list of all available connection types.
   */
  connectionTypes() {
    return this.fetch(["connection-types"]);
  }

  /**
   * Create a new key.
   */
  createKey() {
    return this.fetch(["key"], {method: "POST"});
  }

  /**
   * Delete the current key.
   */
  deleteKey() {
    return this.fetch(["key"], {method: "DELETE"});
  }

  /**
   * Get the current key.
   */
  getKey() {
    return this.fetch(["key"]);
  }
}

function encodePath(path) {
  let out = [];

  for (let part of path) {
    out.push(encodeURIComponent(part));
  }

  return out.join("/");
}