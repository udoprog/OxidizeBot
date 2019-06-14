import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";
import * as th from "react-bootstrap-typeahead";

export class Typeahead extends Base {
  constructor(optional, value, options, what = "thing") {
    super(optional);
    this.value = value;
    this.options = options;
    this.what = what;
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
      if (e.length === 0) {
        return;
      }

      let option = e[0];
      let value = this.value.construct(option.value);
      parentOnChange(value);
    };

    let current = this.options.find(o => o.value === value);

    return (
      <th.Typeahead
        id="select"
        labelKey="title"
        value={value}
        options={this.options}
        placeholder={`Choose a ${this.what}...`}
        defaultInputValue={current.title}
        onChange={onChange}
      />
    );
  }

  hasEditControl() {
    return false;
  }
}