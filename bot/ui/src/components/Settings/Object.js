import React from "react";
import {Button, InputGroup, Form} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {Base} from "./Base";

export class Object extends Base {
  constructor(optional, fields) {
    super(optional);
    this.fields = fields;
  }

  default() {
    let o = {};

    for (let f of this.fields) {
      o[f.field] = f.control.default();
    }

    return o;
  }

  construct(value) {
    let o = {};

    for (let f of this.fields) {
      o[f.field] = f.control.construct(value[f.field]);
    }

    return o;
  }

  serialize(values) {
    let o = {};

    for (let f of this.fields) {
      o[f.field] = f.control.serialize(values[f.field]);
    }

    return o;
  }

  render(values, parentOnChange) {
    return this.fields.map(f => {
      let value = values[f.field];

      let onChange = update => {
        let newValues = {...values};
        newValues[f.field] = update;
        parentOnChange(newValues)
      };

      return (
        <Form.Group key={f.field}>
          <Form.Label>{f.title}</Form.Label>
          <Form.Text>{f.control.render(value, onChange)}</Form.Text>
        </Form.Group>
      );
    });
  }

  editControl() {
    let editControls = {};

    for (let f of this.fields) {
      editControls[f.field] = f.control.editControl();
    }

    return new EditObject(this.optional, this.fields, editControls);
  }

  edit(values) {
    let o = {};

    for (let f of this.fields) {
      o[f.field] = f.control.edit(values[f.field]);
    }

    return o;
  }
}

class EditObject {
  constructor(optional, fields, editControls) {
    this.optional = optional;
    this.fields = fields;
    this.editControls = editControls;
  }

  validate(values) {
    return this.fields.every(f => this.editControls[f.field].validate(values[f.field]));
  }

  save(values) {
    let o = {};

    for (let f of this.fields) {
      o[f.field] = this.editControls[f.field].save(values[f.field]);
    }

    return o;
  }

  render(values, parentOnChange, _isValid) {
    return (
      <Form>
        {this.fields.map(f => {
          let value = values[f.field];

          let onChange = update => {
            let newValues = {...values};
            newValues[f.field] = update;
            parentOnChange(newValues)
          };

          let control = this.editControls[f.field];
          let isValid = control.validate(value);

          return (
            <Form.Group key={f.field}>
              <Form.Label>{f.title}</Form.Label>
              <Form.Text>{control.render(value, onChange, isValid)}</Form.Text>
            </Form.Group>
          );
        })}
      </Form>
    );
  }
}