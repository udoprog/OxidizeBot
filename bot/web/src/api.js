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
}