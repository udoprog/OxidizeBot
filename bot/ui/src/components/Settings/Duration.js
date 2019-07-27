import React from "react";
import {Form, InputGroup, Col} from "react-bootstrap";
import {Base} from "./Base";

const DURATION_REGEX = /^((\d+)d)?((\d+)h)?((\d+)m)?((\d+)s)?$/;

export class Duration extends Base {
  constructor(optional) {
    super(optional);
  }

  /**
   * Parse the given duration.
   *
   * @param {string} input input to parse.
   */
  static parse(input) {
    let m = DURATION_REGEX.exec(input);

    if (!m) {
      throw new Error(`Bad duration: ${input}`);
    }

    let days = 0;
    let hours = 0;
    let minutes = 0;
    let seconds = 0;

    if (!!m[2]) {
      days = parseInt(m[2]);
    }

    if (!!m[4]) {
      hours = parseInt(m[4]);
    }

    if (!!m[6]) {
      minutes = parseInt(m[6]);
    }

    if (!!m[8]) {
      seconds = parseInt(m[8]);
    }

    return {days, hours, minutes, seconds};
  }

  default() {
    return {days: 0, hours: 0, minutes: 0, seconds: 1};
  }

  construct(data) {
    return Duration.parse(data);
  }

  /**
   * Serialize to remote representation.
   */
  serialize(value) {
    return this.convertToString(value);
  }

  render(value) {
    let text = this.convertToString(value);

    return (
      <Form.Control size="sm" value={text} disabled={true} />
    );
  }

  editControl() {
    return new EditDuration();
  }

  edit(value) {
    return value;
  }

  /**
   * Convert the duration into a string.
   */
  convertToString(value) {
    let nothing = true;
    let s = "";

    if (value.days > 0) {
      nothing = false;
      s += `${value.days}d`;
    }

    if (value.hours > 0) {
      nothing = false;
      s += `${value.hours}h`;
    }

    if (value.minutes > 0) {
      nothing = false;
      s += `${value.minutes}m`;
    }

    if (value.seconds > 0 || nothing) {
      s += `${value.seconds}s`;
    }

    return s;
  }
}

class EditDuration {
  validate(value) {
    return (
      value.days >= 0 &&
      value.hours >= 0 && value.hours < 24 &&
      value.minutes >= 0 && value.minutes < 60 &&
      value.seconds >= 0 && value.seconds < 60
    );
  }

  save(value) {
    return Object.assign(value, {});
  }

  render(value, onChange, _isValid) {
    let days = this.digitControl(
      value.days, "d", v => onChange(Object.assign(value, {days: v})), _ => true
    );
    let hours = this.digitControl(
      value.hours, "h", v => onChange(Object.assign(value, {hours: v})), v => v >= 0 && v < 24
    );
    let minutes = this.digitControl(
      value.minutes, "m", v => onChange(Object.assign(value, {minutes: v})), v => v >= 0 && v < 60
    );
    let seconds = this.digitControl(
      value.seconds, "s", v => onChange(Object.assign(value, {seconds: v})), v => v >= 0 && v < 60
    );

    return [
      days,
      hours,
      minutes,
      seconds
    ];
  }

  digitControl(value, suffix, onChange, validate) {
    var isValid = validate(value);

    return [
      <Form.Control key={suffix} type="number" value={value} isInvalid={!isValid} onChange={
        e => {
          onChange(parseInt(e.target.value) || 0);
        }
      } />,

      <InputGroup.Append key={`${suffix}-append`}>
        <InputGroup.Text>{suffix}</InputGroup.Text>
      </InputGroup.Append>
    ];
  }
}