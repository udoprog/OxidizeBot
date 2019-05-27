import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class String extends Base {
  constructor(optional) {
    super(optional);
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
    return new EditString();
  }

  edit(value) {
    return value;
  }
}

class EditString {
  validate(value) {
    return true;
  }

  save(value) {
    return value;
  }

  render(_isValid, value, onChange) {
    return <Form.Control size="sm" type="value" value={value} onChange={e => onChange(e.target.value)} />
  }
}