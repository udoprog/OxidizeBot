import React from "react";
import {Button} from "react-bootstrap";
import {True, False} from "../../utils";
import {Base} from "./Base";

export class BooleanType {
  constructor(optional) {
    this.optional = optional;
  }

  default() {
    return false;
  }

  construct(value) {
    return {
      control: new Boolean(this.optional),
      value,
    };
  }
}

export class Boolean extends Base {
  constructor(optional) {
    super(optional);
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

  serialize(value) {
    return value;
  }
}