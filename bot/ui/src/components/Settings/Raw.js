import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class Raw extends Base {
  constructor(optional) {
    super(optional);
  }

  default() {
    return {};
  }

  construct(value) {
    return value;
  }

  serialize(data) {
    return data;
  }

  render(data) {
    return <code>{JSON.stringify(data)}</code>;
  }

  editControl() {
    return new EditRaw();
  }

  edit(data) {
    return JSON.stringify(data);
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
    return JSON.parse(value);
  }

  render(isValid, value, onChange) {
    return <Form.Control size="sm" type="value" isInvalid={!isValid} value={value} onChange={
      e => {
        onChange(e.target.value);
      }
    } />
  }
}