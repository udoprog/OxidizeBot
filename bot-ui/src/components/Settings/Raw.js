import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";
import YAML from 'yaml'

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
    return YAML.stringify(data);
  }
}

class EditRaw {
  constructor(value) {
    this.value = value;
  }

  validate(value) {
    try {
      YAML.parse(value);
      return true;
    } catch(e) {
      return false;
    }
  }

  save(value) {
    return YAML.parse(value);
  }

  render(value, onChange, isValid) {
    return <Form.Control as="textarea" rows={5} size="sm" type="value" isInvalid={!isValid} value={value} onChange={
      e => {
        onChange(e.target.value);
      }
    } />
  }
}