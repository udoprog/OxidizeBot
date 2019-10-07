import React from "react";
import {Form} from "react-bootstrap";
import {Base} from "./Base";

export class Text extends Base {
  constructor(optional) {
    super(optional);
  }

  default() {
    return "";
  }

  construct(value) {
    return value;
  }

  serialize(value) {
    return value;
  }

  render(value) {
    return <pre className="settings-text"><code>{value}</code></pre>;
  }

  editControl() {
    return new EditString();
  }

  edit(value) {
    return value;
  }
}

class EditString {
  validate(value) {
    return true;
  }

  save(value) {
    return value;
  }

  render(value, onChange, _isValid) {
    return <Form.Control size="sm" as="textarea" value={value} onChange={e => onChange(e.target.value)} />
  }
}