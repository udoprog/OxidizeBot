import React from "react";
import {Form, InputGroup} from "react-bootstrap";
import {Base} from "./Base";

export class PercentageType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return 0;
  }

  construct(data) {
    return {
      control: new Percentage(this.optional),
      value: data,
    };
  }
}

class EditPercentage {
  validate(value) {
    let n = parseInt(value);

    if (isNaN(n)) {
      return false;
    }

    return n >= 0;
  }

  save(value) {
    return {
      control: new Percentage(),
      value: parseInt(value) || 0,
    };
  }

  control(isValid, value, onChange) {
    return (
      <InputGroup size="sm">
        <Form.Control type="number" isInvalid={!isValid} value={value} onChange={
          e => {
            onChange(e.target.value);
          }
        } />
        <InputGroup.Append>
          <InputGroup.Text>%</InputGroup.Text>
        </InputGroup.Append>
      </InputGroup>
    );
  }
}

class Percentage extends Base {
  constructor(optional) {
    super(optional);
  }

  static parse(input) {
    return {
      render: new Percentage(),
      value: parseInt(input) || 0,
    };
  }

  render(value) {
    return `${value}%`;
  }

  edit(editValue) {
    return {
      edit: new EditPercentage(),
      editValue: editValue.toString(),
    };
  }

  serialize(value) {
    return value;
  }
}