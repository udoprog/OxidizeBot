import React from "react";

const DURATION_REGEX = /^((\d+)h)?((\d+)m)?((\d+)s)?$/;

export class Duration {
  constructor(hours, minutes, seconds) {
    this.hours = hours;
    this.minutes = minutes;
    this.seconds = seconds;
  }

  /**
   * Validate the input.
   */
  static validate(text) {
    return DURATION_REGEX.test(text);
  }

  /**
   * Parse the given duration.
   *
   * @param {string} input input to parse.
   */
  static parse(input) {
    let m = DURATION_REGEX.exec(input);

    if (!m) {
      throw new Error(`bad duration: ${input}`);
    }

    let hours = 0;
    let minutes = 0;
    let seconds = 0;

    if (!!m[2]) {
      hours = parseInt(m[2]);
    }

    if (!!m[4]) {
      minutes = parseInt(m[4]);
    }

    if (!!m[6]) {
      seconds = parseInt(m[6]);
    }

    return new Duration(hours, minutes, seconds);
  }

  /**
   * Serialize to remote representation.
   */
  serialize() {
    return this.toString();
  }

  /**
   * Convert the duration into a string.
   */
  toString() {
    let nothing = true;
    let s = "";

    if (this.hours > 0) {
      nothing = false;
      s += `${this.hours}h`;
    }

    if (this.minutes > 0) {
      nothing = false;
      s += `${this.minutes}m`;
    }

    if (this.seconds > 0 || nothing) {
      s += `${this.seconds}s`;
    }

    return s;
  }

  type() {
    return Duration;
  }
}

export class Number {
  constructor(data) {
    this.data = data;
  }

  static validate(text) {
    return !isNaN(parseInt(text));
  }

  static parse(input) {
    let data = JSON.parse(input);

    if (typeof data !== "number") {
      throw new Error("expected number");
    }

    return new Number(data);
  }

  serialize() {
    return this.data;
  }

  toString() {
    return this.data.toString();
  }

  type() {
    return Number;
  }
}

export class Boolean {
  constructor(data) {
    this.data = data;
  }

  static validate(text) {
    switch (text) {
      case "true":
      case "false":
        return true;
      default:
        return false;
    }
  }

  static parse(input) {
    let data = JSON.parse(input);

    if (typeof data !== "boolean") {
      throw new Error("expected boolean");
    }

    return new Boolean(data);
  }

  serialize() {
    return this.data;
  }

  toString() {
    return this.data.toString();
  }

  type() {
    return Raw;
  }
}

export class Raw {
  constructor(data) {
    this.data = data;
  }

  static validate(text) {
    try {
      JSON.parse(text);
      return true;
    } catch(e) {
      return false;
    }
  }

  static parse(data) {
    return new Raw(JSON.parse(data))
  }

  serialize() {
    return this.data;
  }

  toString() {
    return JSON.stringify(this.data);
  }

  type() {
    return Raw;
  }
}