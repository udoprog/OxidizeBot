export class Api {
  constructor(url) {
    this.url = url;
  }

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
   * Get things that requires authentication.
   */
  auth() {
    return this.fetch("auth");
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
  settings() {
    return this.fetch("settings");
  }

  /**
   * Delete a setting.
   *
   * @param {string} key the key of the setting to delete.
   */
  deleteSetting(key) {
    key = settingsKey(key);

    return this.fetch(`setting/${key}`, {
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

    return this.fetch(`setting/${key}`, {
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
}

function encodePath(path) {
  var out = [];

  for (var part of path) {
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
  var parts = key.split("/");
  var out = [];

  for (var part of parts) {
    out.push(encodeURIComponent(part));
  }

  return out.join("/");
}