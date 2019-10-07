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

  render(values, parentOnChange) {
    return (
      <div>
        {values.map((value, key) => {
          let onChange = update => {
            let newValues = values.slice();
            newValues[key] = update;
            parentOnChange(newValues);
          };

          return <div key={key} className="mb-3">{this.control.render(value, onChange)}</div>;
        })}
      </div>
    );
  }

  editControl() {
    return new EditSet(this.optional, this.control, this.control.editControl());
  }

  edit(values) {
    return values.map(value => this.control.edit(value));
  }

  isSingular() {
    return false;
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

  render(values, onChange, _isValid) {
    let add = () => {
      let newValues = values.slice();
      let value = this.control.edit(this.control.default());
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

          let control = this.editControl.render(editValue, v => {
            let newValues = values.slice();
            newValues[key] = v;
            onChange(newValues);
          }, isValid);

          if (this.control.isSingular()) {
            return (
              <InputGroup key={key} className="mb-1">
                {control}
                <InputGroup.Append>
                  <Button size="sm" variant="danger" onClick={remove(key)}><FontAwesomeIcon icon="minus" /></Button>
                </InputGroup.Append>
              </InputGroup>
            );
          } else {
            return [
              <InputGroup key={key} className="mb-1">
                {control}
              </InputGroup>,

              <Button className="mb-2" key={`${key}-delete`} size="sm" variant="danger" onClick={remove(key)}><FontAwesomeIcon icon="minus" /></Button>
            ];
          }
        })}

        <div>
          <Button size="sm" variant="primary" onClick={add}><FontAwesomeIcon icon="plus" /></Button>
        </div>
      </div>
    );
  }
}