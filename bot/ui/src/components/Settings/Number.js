import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class Number extends Base {
  constructor(optional) {
    super(optional);
  }

  default() {
    return 0;
  }

  construct(data) {
    return data;
  }

  serialize(value) {
    return value;
  }

  render(value) {
    return (
      <Form.Control size="sm" value={value.toString()} disabled={true} />
    );
  }

  editControl() {
    return new EditNumber();
  }

  edit(value) {
    return value.toString();
  }
}

class EditNumber {
  validate(value) {
    return !isNaN(parseInt(value));
  }

  save(value) {
    return parseInt(value);
  }

  render(value, onChange, isValid) {
    return <Form.Control size="sm" type="number" isInvalid={!isValid} value={value} onChange={
      e => {
        onChange(e.target.value);
      }
    } />
  }
}