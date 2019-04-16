export class Api {
  constructor(url) {
    this.url = url;
  }

  fetch(path, data = {}) {
    return fetch(`${this.url}/${path}`, data).then((r) => {
      if (!r.ok) {
        throw Error(`got bad status code: ${r.status}`);
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
}