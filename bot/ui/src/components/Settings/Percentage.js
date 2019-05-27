import React from "react";
import {Form, InputGroup} from "react-bootstrap";
import {Base} from "./Base";

export class Percentage extends Base {
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
    return `${value}%`;
  }

  editControl() {
    return new EditPercentage();
  }

  edit(value) {
    return value.toString();
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
    return parseInt(value) || 0;
  }

  render(isValid, value, onChange) {
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