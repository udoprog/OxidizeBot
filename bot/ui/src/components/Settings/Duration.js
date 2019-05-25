import React from "react";
import {Form, InputGroup, Col} from "react-bootstrap";
import {Base} from "./Base";

const DURATION_REGEX = /^((\d+)d)?((\d+)h)?((\d+)m)?((\d+)s)?$/;

export class DurationType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return {days: 0, hours: 0, minutes: 0, seconds: 1};
  }

  construct(data) {
    return Duration.parse(this.optional, data);
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
    return {
      control: new Duration(),
      value: Object.assign(value, {}),
    };
  }

  control(_isValid, value, onChange) {
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

    return (
      <Form.Row>
        <Col>
          {days}
        </Col>

        <Col>
          {hours}
        </Col>

        <Col>
          {minutes}
        </Col>

        <Col>
          {seconds}
        </Col>
      </Form.Row>
    );
  }

  digitControl(value, suffix, onChange, validate) {
    var isValid = validate(value);

    return (
      <InputGroup size="sm">
        <Form.Control type="number" value={value} isInvalid={!isValid} onChange={
          e => {
            onChange(parseInt(e.target.value) || 0);
          }
        } />

        <InputGroup.Append>
          <InputGroup.Text>{suffix}</InputGroup.Text>
        </InputGroup.Append>
      </InputGroup>
    );
  }
}

class Duration extends Base {
  constructor(optional) {
    super(optional);
  }

  /**
   * Parse the given duration.
   *
   * @param {string} input input to parse.
   */
  static parse(optional, input) {
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

    return {
      control: new Duration(optional),
      value: {days, hours, minutes, seconds},
    };
  }

  render(value) {
    return <code>{this.convertToString(value)}</code>;
  }

  edit(editValue) {
    return {
      edit: new EditDuration(),
      editValue,
    };
  }

  /**
   * Serialize to remote representation.
   */
  serialize(value) {
    return this.convertToString(value);
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