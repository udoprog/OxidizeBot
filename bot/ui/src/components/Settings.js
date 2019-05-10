import React from "react";
import {Spinner} from "../utils.js";
import {Form, Button, Alert, Table, ButtonGroup} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as types from "./Settings/Types.js";

const SECRET_PREFIX = "secrets/";

function ConfirmButtons(props) {
  let confirmDisabled = props.confirmDisabled || false;

  return (
    <ButtonGroup>
      <Button title={`Cancel ${props.what}`} variant="primary" size="sm" onClick={e => props.onCancel(e)}>
        <FontAwesomeIcon icon="window-close" />
      </Button>
      <Button title={`Confirm ${props.what}`} disabled={confirmDisabled} variant="danger" size="sm" onClick={e => props.onConfirm(e)}>
        <FontAwesomeIcon icon="check-circle" />
      </Button>
    </ButtonGroup>
  );
}

export default class Settings extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
      // set to the key of the setting currently being deleted.
      deleteKey: null,
      // set to the key of the setting currently being edited.
      editKey: null,
      // the value currently being edited.
      editValue: null,
    };
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list()
  }

  /**
   * Refresh the list of settings.
   */
  list() {
    this.setState({
      loading: true,
    });

    this.api.settings()
      .then(data => {
        data = data.map(d => {
          if (typeof d !== "object" || d.type === null) {
            return {
              key: d.key,
              value: new types.Raw(d.value),
            };
          }

          switch (d.type.id) {
            case "duration":
              return {
                key: d.key,
                value: types.Duration.parse(d.value),
              };
            case "boolean":
              return {
                key: d.key,
                value: new types.Boolean(d.value),
              };
            case "string":
              return {
                key: d.key,
                value: new types.String(d.value),
              };
            case "number":
              return {
                key: d.key,
                value: new types.Number(d.value),
              };
            default:
              return {
                key: d.key,
                value: new types.Raw(d.value),
              };
          }
        });

        this.setState({
          loading: false,
          error: null,
          data,
        });
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to request after streams: ${e}`,
          data: null,
        });
      });
  }

  /**
   * Delete the given setting.
   *
   * @param {string} key key of the setting to delete.
   */
  delete(key) {
    this.setState({
      loading: true,
      deleteKey: null
    });

    this.api.deleteSetting(key)
      .then(() => {
        return this.list();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to delete setting: ${e}`,
        });
      });
  }

  /**
   * Edit the given setting.
   *
   * @param {string} key key of the setting to edit.
   * @param {string} value the new value to edit it to.
   */
  edit(key, value) {
    this.setState({
      loading: true,
      editKey: null,
      editValue: null,
    });

    this.api.editSetting(key, value.serialize())
      .then(() => {
        return this.list();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to edit setting: ${e}`,
        });
      });
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let refresh = null;
    let loading = null;

    if (this.state.loading) {
      loading = <Spinner />;
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh right" />;
    } else {
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh clickable right" onClick={() => this.list()} />;
    }

    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No Settings!
          </Alert>
        );
      } else {
        content = (
          <Table responsive="sm">
            <thead>
              <tr>
                <th>Key</th>
                <th width="99%">Value</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.map((setting, id) => {
                let isSecret = setting.key.startsWith(SECRET_PREFIX);

                let buttons = (
                  <ButtonGroup>
                    <Button size="sm" variant="danger" className="action" onClick={() => this.setState({
                      deleteKey: setting.key,
                    })}>
                      <FontAwesomeIcon icon="trash" />
                    </Button>
                    <Button size="sm" variant="info" className="action" onClick={() => this.setState({
                      editKey: setting.key,
                      editValue: setting.value.edit(),
                    })}>
                      <FontAwesomeIcon icon="edit" />
                    </Button>
                  </ButtonGroup>
                );

                let value = null;

                if (isSecret) {
                  value = <b title="Secret value, only showed when editing">****</b>;
                } else {
                  value = <code>{setting.value.toString()}</code>;
                }

                if (this.state.deleteKey === setting.key) {
                  buttons = <ConfirmButtons
                    what="deletion"
                    onConfirm={() => this.delete(this.state.deleteKey)}
                    onCancel={() => {
                      this.setState({
                        deleteKey: null,
                      })
                    }}
                  />;
                }

                if (this.state.editKey === setting.key) {
                  let isValid = this.state.editValue.validate();

                  let save = (e) => {
                    e.preventDefault();

                    if (isValid) {
                      let value = this.state.editValue.save();
                      this.edit(this.state.editKey, value);
                    }

                    return false;
                  };

                  let control = this.state.editValue.control(isValid, editValue => {
                    this.setState({
                      editValue
                    });
                  });

                  value = (
                    <Form onSubmit={e => save(e)}>
                      {control}
                    </Form>
                  );

                  buttons = <ConfirmButtons
                    what="edit"
                    confirmDisabled={!isValid}
                    onConfirm={e => save(e)}
                    onCancel={() => {
                      this.setState({
                        editKey: null,
                        editValue: null,
                      })
                    }}
                  />;
                }

                return (
                  <tr key={id}>
                    <td className="settings-key">{setting.key}</td>
                    <td>{value}</td>
                    <td align="right">{buttons}</td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <div>
        <h2>
          Settings
          {refresh}
        </h2>
        {error}
        {content}
        {loading}
      </div>
    );
  }
}