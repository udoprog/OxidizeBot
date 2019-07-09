export class Api {
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
   * Get version information.
   */
  version() {
    return this.fetch(["version"]);
  }

  /**
   * Get things that requires authentication.
   */
  authPending() {
    return this.fetch(["auth", "pending"]);
  }

  /**
   * Get a list of devices.
   */
  devices() {
    return this.fetch("devices");
  }

  /**
   * Set the device to play back from.
   *
   * @param {string} id the id of the device to set.
   */
  setDevice(id) {
    return this.fetch(`device/${id}`, {
      method: "POST",
    });
  }

  /**
   * Get a list of devices.
   */
  afterStreams() {
    return this.fetch("after-streams");
  }

  /**
   * Delete an after stream.
   *
   * @param {number} id id of after stream to delete.
   */
  deleteAfterStream(id) {
    return this.fetch(`after-stream/${id}`, {
      method: "DELETE",
    });
  }

  /**
   * Get the list of settings.
   */
  settings(filter = {}) {
    let queries = [];

    if (!!filter.key) {
      let value = filter.key.join(",");
      queries.push(`key=${value}`);
    }

    if (!!filter.prefix) {
      let value = filter.prefix.join(",");
      queries.push(`prefix=${value}`);
    }

    if (filter.feature !== undefined) {
      queries.push(`feature=true`);
    }

    let query = "";

    if (queries.length > 0) {
      query = queries.join("&");
      query = `?${query}`;
    }

    return this.fetch(
      `settings${query}`
    );
  }

  /**
   * Delete a setting.
   *
   * @param {string} key the key of the setting to delete.
   */
  deleteSetting(key) {
    key = settingsKey(key);

    return this.fetch(`settings/${key}`, {
      method: "DELETE",
    });
  }

  /**
   * Edit the given setting.
   *
   * @param {string} key the key to edit
   * @param {any} value the value to edit
   */
  editSetting(key, value) {
    key = settingsKey(key);

    return this.fetch(`settings/${key}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        value: value,
      }),
    });
  }

  /**
   * Export balances.
   */
  exportBalances() {
    return this.fetch("balances");
  }

  /**
   * Import balances.
   */
  importBalances(balances) {
    return this.fetch("balances", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(balances),
    });
  }

  /**
   * Get information on the current user.
   */
  current() {
    return this.fetch("current");
  }

  aliases(channel) {
    return this.fetch(["aliases", channel]);
  }

  /**
   * Edit the disabled state of an alias.
   *
   * @param {object} key key of the alias to edit
   * @param {bool} disabled set the alias disabled or not
   */
  aliasesEditDisabled(key, disabled) {
    return this.fetch(["aliases", key.channel, key.name, "disabled"], {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({disabled}),
    });
  }

  /**
   * List all commands from a channel.
   */
  commands(channel) {
    return this.fetch(["commands", channel]);
  }

  /**
   * Edit the disabled state of a command.
   *
   * @param {object} key key of the command to edit
   * @param {bool} disabled set the command disabled or not
   */
  commandsEditDisabled(key, disabled) {
    return this.fetch(["commands", key.channel, key.name, "disabled"], {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({disabled}),
    });
  }

  promotions(channel) {
    return this.fetch(["promotions", channel]);
  }

  /**
   * Edit the disabled state of a promotion.
   *
   * @param {object} key key of the promotion to edit
   * @param {bool} disabled set the promotion disabled or not
   */
  promotionsEditDisabled(key, disabled) {
    return this.fetch(["promotions", key.channel, key.name, "disabled"], {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({disabled}),
    });
  }

  themes(channel) {
    return this.fetch(["themes", channel]);
  }

  /**
   * Edit the disabled state of an theme.
   *
   * @param {object} key key of the theme to edit
   * @param {bool} disabled set the theme disabled or not
   */
  themesEditDisabled(key, disabled) {
    return this.fetch(["themes", key.channel, key.name, "disabled"], {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({disabled}),
    });
  }

  /**
   * Get a list of all available scopes.
   */
  authScopes() {
    return this.fetch(["auth", "scopes"]);
  }

  /**
   * Get a list of all available roles.
   */
  authRoles() {
    return this.fetch(["auth", "roles"]);
  }

  /**
   * Get a list of all enabled grants.
   */
  authGrants() {
    return this.fetch(["auth", "grants"]);
  }

  /**
   * Insert a grant into the database.
   */
  authInsertGrant(auth) {
    return this.fetch(["auth", "grants"], {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(auth),
    });
  }

  /**
   * Delete a grant from the database.
   */
  authDeleteGrant(scope, role) {
    return this.fetch(["auth", "grants", scope, role], {
      method: "DELETE",
    });
  }

  /**
   * Get all existing chat messages.
   */
  chatMessages() {
    return this.fetch(["chat", "messages"]);
  }
}

function encodePath(path) {
  let out = [];

  for (let part of path) {
    out.push(encodeURIComponent(part));
  }

  return out.join("/");
}

/**
 * Encode the URI for a settings key.
 *
 * @param {string} key
 */
function settingsKey(key) {
  let parts = key.split("/");
  let out = [];

  for (let part of parts) {
    out.push(encodeURIComponent(part));
  }

  return out.join("/");
}