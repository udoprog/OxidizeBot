import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class NumberType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return 0;
  }

  construct(data) {
    return {
      control: new Number(this.optional),
      value: data,
    };
  }
}

class EditNumber {
  validate(value) {
    return !isNaN(parseInt(value));
  }

  save(value) {
    return {
      control: new Number(),
      value,
    };
  }

  control(isValid, value, onChange) {
    return <Form.Control size="sm" type="number" isInvalid={!isValid} value={value} onChange={
      e => {
        onChange(e.target.value);
      }
    } />
  }
}

class Number extends Base {
  constructor(optional) {
    super(optional);
  }

  static parse(input) {
    return {
      render: new Number(),
      value: parseInt(input) || 0,
    };
  }

  render(value) {
    return value.toString();
  }

  edit(editValue) {
    return {
      edit: new EditNumber(),
      editValue: editValue.toString(),
    };
  }

  serialize(value) {
    return value;
  }
}