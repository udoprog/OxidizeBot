import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class String extends Base {
  constructor(optional, format, placeholder) {
    super(optional);
    this.format = format;
    this.placeholder = placeholder;
  }

  default() {
    return "";
  }

  construct(value) {
    return value;
  }

  serialize(value) {
    return value;
  }

  render(value) {
    return <code>{value}</code>;
  }

  editControl() {
    return new EditString(this.format, this.placeholder);
  }

  edit(value) {
    return value;
  }
}

class EditString {
  constructor(format, placeholder) {
    this.format = format;
    this.placeholder = placeholder;
  }

  validate(value) {
    return this.format.validate(value);
  }

  save(value) {
    return value;
  }

  render(isValid, value, onChange) {
    return <Form.Control
      size="sm"
      type="value"
      placeholder={this.placeholder}
      isInvalid={!isValid}
      value={value}
      onChange={e => onChange(e.target.value)} />;
  }
}