import React from "react";
import {Form} from "react-bootstrap";

const DURATION_REGEX = /^((\d+)h)?((\d+)m)?((\d+)s)?$/;

class EditDuration {
  validate() {
    return DURATION_REGEX.test(this.value);
  }

  save() {
    return Duration.parse(this.value);
  }

  control(isValid, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class Duration {
  constructor(hours, minutes, seconds) {
    this.hours = hours;
    this.minutes = minutes;
    this.seconds = seconds;
  }

  /**
   * Parse the given duration.
   *
   * @param {string} input input to parse.
   */
  static parse(input) {
    let m = DURATION_REGEX.exec(input);

    if (!m) {
      return null;
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

  edit() {
    return new EditDuration(this.toString());
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
}

class EditNumber {
  constructor(value) {
    this.value = value;
  }

  validate() {
    return !isNaN(parseInt(this.value));
  }

  save() {
    return Number.parse(this.value);
  }

  control(isValid, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class Number {
  constructor(data) {
    this.data = data;
  }

  static parse(input) {
    let data = JSON.parse(input);

    if (typeof data !== "number") {
      throw new Error("expected number");
    }

    return new Number(data);
  }

  edit() {
    return new EditNumber(this.toString());
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

class EditBoolean {
  constructor(value) {
    this.value = value;
  }

  validate() {
    switch (this.value) {
      case "true":
      case "false":
        return true;
      default:
        return false;
    }
  }

  save() {
    return Boolean.parse(this.value);
  }

  control(isValid, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class Boolean {
  constructor(data) {
    this.data = data;
  }

  static parse(input) {
    let data = JSON.parse(input);

    if (typeof data !== "boolean") {
      throw new Error("expected boolean");
    }

    return new Boolean(data);
  }

  edit() {
    return new EditBoolean(this.toString());
  }

  serialize() {
    return this.data;
  }

  toString() {
    return this.data.toString();
  }
}

class EditString {
  constructor(value) {
    this.value = value;
  }

  validate() {
    return true;
  }

  save() {
    return String.parse(this.value);
  }

  control(isValid, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class String {
  constructor(data) {
    this.data = data;
  }

  static parse(input) {
    if (typeof input !== "string") {
      throw new Error("expected string");
    }

    return new String(data);
  }

  edit() {
    return new EditString(this.toString());
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

class EditRaw {
  constructor(value) {
    this.value = value;
  }

  validate() {
    try {
      JSON.parse(this.value);
      return true;
    } catch(e) {
      return false;
    }
  }

  save() {
    return Raw.parse(this.value);
  }

  control(isValid, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class Raw {
  constructor(data) {
    this.data = data;
  }

  static parse(data) {
    return new Raw(JSON.parse(data))
  }

  edit() {
    return new EditRaw(this.toString());
  }

  serialize() {
    return this.data;
  }

  toString() {
    return JSON.stringify(this.data);
  }
}