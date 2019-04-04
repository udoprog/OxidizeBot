import React from "react";

/**
 * Format duration in a human-readable way.
 * @param {*} duration
 */
export function formatDuration(duration) {
  let seconds = duration % 60;
  let minutes = Math.floor(duration / 60);

  return pad(minutes, 2) + ":" + pad(seconds, 2);

  function pad(num, size) {
    var s = num + "";

    while (s.length < size) {
      s = "0" + s;
    }

    return s;
  }
}

/**
 * Get a percentage form a part and a total.
 *
 * @param {number} part
 * @param {number} total
 */
export function percentage(part, total) {
  if (part === total) {
    return 100;
  }

  return Math.round((part / total) * 10000) / 100;
}

/**
 * Get the current URL to connect to.
 */
export function websocketUrl(path) {
  var loc = window.location;
  var scheme = "ws";

  if (loc.protocol === "https:") {
    scheme = "wss";
  }

  return `${scheme}://${loc.host}/${path}`;
}

/**
 * Get the current URL to connect to.
 */
export function apiUrl() {
  var loc = window.location;
  var scheme = "http";

  if (loc.protocol === "https:") {
    scheme = "https";
  }

  let path = loc.pathname.split("/");
  path = path.slice(0, path.length - 1).join("/");

  return `${scheme}://${loc.host}${path}/api`;
}

/**
 * Pick the image best suited for album art.
 */
export function pickAlbumArt(images, smaller) {
  for (let i = 0; i < images.length; i++) {
    let image = images[i];

    if (image.width <= smaller && image.height <= smaller) {
      return image;
    }
  }

  return null;
}

/**
 * Pick the image best suited for album art.
 */
export function pickArtist(artists) {
  if (artists.length == 0) {
    return null;
  }

  return artists[0];
}

/**
 * A simple spinner component.
 */
export function Spinner() {
  return (
    <div className="spinner">
      <div className="bounce1"></div>
      <div className="bounce2"></div>
      <div className="bounce3"></div>
    </div>
  );
}