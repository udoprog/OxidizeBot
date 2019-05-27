import React from "react";
import {Button, InputGroup} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {Base} from "./Base";

export class Set extends Base {
  constructor(optional, control) {
    super(optional);
    this.control = control;
  }

  default() {
    return [];
  }

  construct(value) {
    return value.map(v => this.control.construct(v));
  }

  serialize(values) {
    return values.map(value => this.control.serialize(value));
  }

  render(values) {
    return (
      <div>
        {values.map((value, key) => <div key={key}>{this.control.render(value)}</div>)}
      </div>
    );
  }

  editControl() {
    return new EditSet(this.optional, this.control, this.control.editControl());
  }

  edit(values) {
    return values.map(value => this.control.edit(value));
  }
}

class EditSet {
  constructor(optional, control, editControl) {
    this.optional = optional;
    this.control = control;
    this.editControl = editControl;
  }

  validate(values) {
    return values.every(value => this.editControl.validate(value));
  }

  save(values) {
    return values.map(value => this.editControl.save(value));
  }

  render(_isValid, values, onChange) {
    let add = () => {
      let newValues = values.slice();
      let value = this.control.construct(this.control.default());
      newValues.push(value);
      onChange(newValues);
    };

    let remove = key => _ => {
      let newValues = values.slice();
      newValues.splice(key, 1);
      onChange(newValues);
    };

    return (
      <div>
        {values.map((editValue, key) => {
          let isValid = this.editControl.validate(editValue);

          let control = this.editControl.render(isValid, editValue, v => {
            let newValues = values.slice();
            newValues[key] = v;
            onChange(newValues);
          });

          return (
            <InputGroup key={key} className="mb-1">
              {control}
              <InputGroup.Append>
                <Button size="sm" variant="danger" onClick={remove(key)}><FontAwesomeIcon icon="minus" /></Button>
              </InputGroup.Append>
            </InputGroup>
          );
        })}

        <div>
          <Button size="sm" variant="primary" onClick={add}><FontAwesomeIcon icon="plus" /></Button>
        </div>
      </div>
    );
  }
}