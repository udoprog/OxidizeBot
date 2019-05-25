import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class StringType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return "";
  }

  construct(value) {
    return {
      control: new String(this.optional),
      value,
    };
  }
}

class EditString {
  validate(value) {
    return true;
  }

  save(value) {
    return {
      control: new String(),
      value,
    };
  }

  control(_isValid, value, onChange) {
    return <Form.Control size="sm" type="value" value={value} onChange={e => onChange(e.target.value)} />
  }
}

class String extends Base {
  constructor(optional) {
    super(optional);
  }

  render(value) {
    return <code>{value}</code>;
  }

  edit(editValue) {
    return {
      edit: new EditString(),
      editValue,
    };
  }

  serialize(value) {
    return value;
  }
}