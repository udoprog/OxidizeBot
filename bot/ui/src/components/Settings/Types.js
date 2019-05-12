import React from "react";
import {Form, Button, InputGroup, Row, Col} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {True, False} from "../../utils";

/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */
export function decode(type) {
  if (type === null) {
    return RawType;
  }

  switch (type.id) {
    case "duration":
      return DurationType;
    case "bool":
      return BooleanType;
    case "string":
      return StringType;
    case "number":
      return NumberType;
    case "set":
      let value = decode(type.value);
      return new SetType(value);
    default:
      return RawType;
  }
}

const DURATION_REGEX = /^((\d+)h)?((\d+)m)?((\d+)s)?$/;

class EditDuration {
  constructor(hours, minutes, seconds) {
    this.hours = hours;
    this.minutes = minutes;
    this.seconds = seconds;
  }

  validate() {
    return this.minutes >= 0 && this.minutes < 60 && this.seconds >= 0 && this.seconds < 60;
  }

  save() {
    return new Duration(this.hours, this.minutes, this.seconds);
  }

  control(_isValid, onChange) {
    let hours = this.digitControl(this.hours, "h", value => this.hours = value, onChange, _ => true);
    let minutes = this.digitControl(this.minutes, "m", value => this.minutes = value, onChange, value => value >= 0 && value < 60);
    let seconds = this.digitControl(this.seconds, "s", value => this.seconds = value, onChange, value => value >= 0 && value < 60);

    return (
      <Row>
        <Col>
          {hours}
        </Col>

        <Col>
          {minutes}
        </Col>

        <Col>
          {seconds}
        </Col>
      </Row>
    );
  }

  digitControl(value, suffix, set, onChange, validate) {
    var isValid = validate(value);

    return (
      <InputGroup size="sm">
        <Form.Control type="number" value={value} isInvalid={!isValid} onChange={
          e => {
            set(parseInt(e.target.value) || 0);
            onChange(this);
          }
        } />

        <InputGroup.Append>
          <InputGroup.Text>{suffix}</InputGroup.Text>
        </InputGroup.Append>
      </InputGroup>
    );
  }
}

class DurationType {
  static default() {
    return new Duration(0, 0, 1);
  }

  static construct(data) {
    return Duration.parse(data);
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

  render() {
    return <code>{this.toString()}</code>;
  }

  edit() {
    return new EditDuration(this.hours, this.minutes, this.seconds);
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
    return <Form.Control size="sm" type="number" isInvalid={!isValid} value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

class NumberType {
  static default() {
    return new Number(0);
  }

  static construct(data) {
    return new Number(data);
  }
}

export class Number {
  constructor(data) {
    this.data = data;
  }

  static parse(input) {
    return new Number(parseInt(input));
  }

  render() {
    return this.toString();
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
    return true;
  }

  save() {
    return new Boolean(this.value);
  }

  control(_isValid, onChange) {
    if (this.value) {
      return <Button title="Toggle to false" size="sm" variant="success" onClick={
        e => {
          this.value = false
          onChange(this);
        }
      }><True /></Button>;
    } else {
      return <Button title="Toggle to true" size="sm" variant="danger" onClick={
        e => {
          this.value = true
          onChange(this);
        }
      }><False /></Button>;
    }
  }
}

export class BooleanType {
  static default() {
    return new Boolean(false);
  }

  static construct(data) {
    return new Boolean(data);
  }
}

export class Boolean {
  constructor(value) {
    this.value = value;
  }

  render() {
    if (this.value) {
      return <Button size="sm" variant="success" disabled><True /></Button>;
    } else {
      return <Button size="sm" variant="danger" disabled><False /></Button>;
    }
  }

  edit() {
    return new EditBoolean(this.value);
  }

  serialize() {
    return this.value;
  }

  toString() {
    return this.value.toString();
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
    return new String(this.value);
  }

  control(_isValid, onChange) {
    return <Form.Control size="sm" type="value" value={this.value} onChange={
      e => {
        this.value = e.target.value;
        onChange(this);
      }
    } />
  }
}

export class StringType {
  static default() {
    return new String("");
  }

  static construct(data) {
    return new String(data);
  }
}

export class String {
  constructor(data) {
    this.data = data;
  }

  render() {
    return <code>{this.toString()}</code>;
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

export class RawType {
  static default() {
    return new Raw({});
  }

  static construct(data) {
    return new Raw(data);
  }
}

export class Raw {
  constructor(data) {
    this.data = data;
  }

  static parse(data) {
    return new Raw(JSON.parse(data))
  }

  render() {
    return <code>{this.toString()}</code>;
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


class EditSet {
  constructor(values, type) {
    this.values = values;
    this.type = type;
  }

  validate() {
    return true;
  }

  save() {
    return new Set(this.values.map(v => v.save()), this.type);
  }

  control(_isValid, onChange) {
    let add = e => {
      this.values.push(this.type.default().edit());
      onChange(this);
    };

    let remove = key => _ => {
      this.values.splice(key, 1);
      onChange(this);
    };

    return (
      <div>
        {this.values.map((v, key) => {
          let isValid = v.validate();

          let e = v.control(isValid, v => {
            onChange(this);
          });

          return (
            <InputGroup key={key} className="mb-1">
              {e}
              <InputGroup.Append>
                <Button size="sm" variant="danger" onClick={remove(key)}><FontAwesomeIcon icon="minus" /></Button>
              </InputGroup.Append>
            </InputGroup>
          );
        })}

        <div>
          <Button size="sm" variant="primary" onClick={add}><FontAwesomeIcon icon="plus" /></Button>
        </div>
      </div>
    );
  }
}

export class SetType {
  constructor(value) {
    this.value = value;
  }

  default() {
    return new Set([], this.value);
  }

  construct(data) {
    return new Set(data.map(d => this.value.construct(d)), this.value);
  }
}

export class Set {
  constructor(values, type) {
    this.values = values;
    this.type = type;
  }

  render() {
    return (
      <div>
        {this.values.map((v, key) => <div key={key}>{v.render()}</div>)}
      </div>
    );
  }

  edit() {
    return new EditSet(this.values.map(v => v.edit()), this.type);
  }

  serialize() {
    return this.values.map(v => v.serialize());
  }
}