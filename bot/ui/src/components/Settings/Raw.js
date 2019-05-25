import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class RawType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return {};
  }

  construct(value) {
    return {
      control: new Raw(this.optional),
      value,
    };
  }
}

class EditRaw {
  constructor(value) {
    this.value = value;
  }

  validate(value) {
    try {
      JSON.parse(value);
      return true;
    } catch(e) {
      return false;
    }
  }

  save(value) {
    return {
      control: new Raw(),
      value: JSON.parse(value),
    };
  }

  control(isValid, value, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={value} onChange={
      e => {
        onChange(e.target.value);
      }
    } />
  }
}

class Raw extends Base {
  constructor(optional) {
    super(optional);
  }

  render(data) {
    return <code>{JSON.stringify(data)}</code>;
  }

  edit(data) {
    return {
      edit: new EditRaw(),
      editValue: JSON.stringify(data),
    };
  }

  serialize(data) {
    return data;
  }
}