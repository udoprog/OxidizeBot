import React from "react";
import {Form, Row, Col} from "react-bootstrap";
import {Base} from "./Base";
import YAML from 'yaml'

export class Oauth2Config extends Base {
  constructor(optional) {
    super(optional);
  }

  default() {
    return {};
  }

  construct(value) {
    return value;
  }

  serialize(data) {
    return data;
  }

  render(data) {
    return [
      <Form.Group key="client-id">
        <Form.Label>Client ID</Form.Label>
        <Form.Control size="sm" disabled={true} value={data.client_id} />
      </Form.Group>,
      <Form.Group key="client-secret">
        <Form.Label>Client Secret</Form.Label>
        <Form.Control size="sm" disabled={true} value={data.client_secret} />
      </Form.Group>
    ];
  }

  editControl() {
    return new EditOauth2Config();
  }

  edit(data) {
    data = Object.assign({}, data);

    if (!data.client_id) {
      data.client_id = "";
    }

    if (!data.client_secret) {
      data.client_secret = "";
    }

    return data;
  }
}

class EditOauth2Config {
  constructor(value) {
    this.value = value;
  }

  validate(value) {
    if (!value.client_id) {
      return false;
    }

    if (!value.client_secret) {
      return false;
    }

    return true;
  }

  save(value) {
    return {
      "client_id": value.client_id,
      "client_secret": value.client_secret,
    };
  }

  render(isValid, value, onChange) {
    let changeclient_id = e => {
      value = Object.assign({}, value);
      value.client_id = e.target.value;
      onChange(value);
    };

    let changeclient_secret = e => {
      value = Object.assign({}, value);
      value.client_secret = e.target.value;
      onChange(value);
    };

    let client_idValid = !!value.client_id;
    let client_secretValid = !!value.client_secret;

    return [
      <Form.Group key="client-id" controlId="client-id">
        <Form.Label>Client ID</Form.Label>
        <Form.Control size="sm" isInvalid={!client_idValid} value={value.client_id} onChange={changeclient_id} />
      </Form.Group>,
      <Form.Group key="client-secret" controlId="client-secret" className="mb-0">
        <Form.Label>Client Secret</Form.Label>
        <Form.Control size="sm" isInvalid={!client_secretValid} value={value.client_secret} onChange={changeclient_secret} />
      </Form.Group>
    ];
  }
}