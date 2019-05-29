import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class Select extends Base {
  constructor(optional, value, options) {
    super(optional);
    this.value = value;
    this.options = options;
  }

  default() {
    return this.value.default();
  }

  construct(value) {
    return this.value.construct(value);
  }

  serialize(value) {
    return this.value.serialize(value);
  }

  render(value, parentOnChange) {
    let onChange = e => {
      let option = this.options[e.target.selectedIndex];
      let value = this.value.construct(option.value);
      parentOnChange(value);
    };

    return (
      <Form.Control as="select" size="sm" type="value" value={value} onChange={onChange}>
        {this.options.map((o, i) => {
          return <option value={o.value} key={i}>{o.title}</option>;
        })}
      </Form.Control>
    );
  }

  hasEditControl() {
    return false;
  }
}