import React from "react";
import {Button} from "react-bootstrap";
import {True, False} from "../../utils";
import {Base} from "./Base";

export class Boolean extends Base {
  constructor(optional) {
    super(optional);
  }

  default() {
    return false;
  }

  construct(value) {
    return value;
  }

  serialize(value) {
    return value;
  }

  render(value, onChange) {
    if (value) {
      return <Button title="Toggle to false" size="sm" variant="success" onClick={() => onChange(false)}><True /></Button>;
    } else {
      return <Button title="Toggle to true" size="sm" variant="danger" onClick={() => onChange(true)}><False /></Button>;
    }
  }

  hasEditControl() {
    return false;
  }
}