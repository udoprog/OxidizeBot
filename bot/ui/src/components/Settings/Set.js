import React from "react";
import {Button, InputGroup} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {Base} from "./Base";

export class SetType {
  constructor(optional, type) {
    this.optional = optional;
    this.type = type;
  }

  default() {
    return [];
  }

  construct(value) {
    return {
      control: new Set(this.optional, this.type),
      value: value.map(v => this.type.construct(v)),
    };
  }
}

class EditSet {
  constructor(type) {
    this.type = type;
  }

  validate(values) {
    return values.every((({edit, editValue}) => edit.validate(editValue)));
  }

  save(values) {
    return {
      control: new Set(this.type),
      value: values.map(({edit, editValue}) => edit.save(editValue)),
    };
  }

  control(_isValid, values, onChange) {
    let add = () => {
      let newValues = values.slice();
      let {control, value} = this.type.construct(this.type.default());
      newValues.push(control.edit(value));
      onChange(newValues);
    };

    let remove = key => _ => {
      let newValues = values.slice();
      newValues.splice(key, 1);
      onChange(newValues);
    };

    return (
      <div>
        {values.map(({edit, editValue}, key) => {
          let isValid = edit.validate(editValue);

          let control = edit.control(isValid, editValue, v => {
            let newValues = values.slice();
            newValues[key].editValue = v;
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

class Set extends Base {
  constructor(optional, type) {
    super(optional);
    this.type = type;
  }

  render(values) {
    return (
      <div>
        {values.map(({control, value}, key) => <div key={key}>{control.render(value)}</div>)}
      </div>
    );
  }

  edit(values) {
    return {
      edit: new EditSet(this.type),
      editValue: values.map(({control, value}) => control.edit(value)),
    };
  }

  serialize(values) {
    return values.map(({control, value}) => control.serialize(value));
  }
}