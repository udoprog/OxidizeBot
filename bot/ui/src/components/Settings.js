import React from "react";
import {Spinner} from "../utils.js";
import {Form, Button, Alert, Table, ButtonGroup, Row, Col} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as types from "./Settings/Types.js";

/**
 * Partition data so that it is displayer per-group.
 */
function partition(data) {
  let def = [];
  let groups = {};

  for (let d of data) {
    let p = d.key.split('/');

    if (p.length === 1) {
      def.push(d);
      continue;
    }

    let rest = p.splice(1).join('/');
    let g = p[0];

    let group = groups[g] || [];

    group.push({
      short: rest,
      data: d,
    });

    groups[g] = group;
  }

  let order = Object.keys(groups);
  order.sort();
  return {order, groups, def};
}

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
      // the controller for the edit.
      edit: null,
      // the value currrently being edited.
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
          let type = types.decode(d.schema.type);
          let {control, value} = type.construct(d.value);

          return {
            key: d.key,
            control,
            value,
            doc: d.schema.doc,
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
  edit(key, {control, value}) {
    this.setState(state => {
      let data = state.data.map(setting => {
        if (setting.key === key) {
          return Object.assign(setting, {control, value});
        }

        return setting;
      });

      return {
        data,
        loading: true,
        editKey: null,
        edit: null,
        editValue: null,
      };
    });

    this.api.editSetting(key, control.serialize(value))
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

  renderSetting(setting, keyOverride = null) {
    let isSecret = setting.key.startsWith(SECRET_PREFIX);

    let editButton = null;
    // onChange handler used for things which support immediate editing.
    let renderOnChange = null;

    if (setting.control.hasEditControl()) {
      let edit = () => {
        let {edit, editValue} = setting.control.edit(setting.value);

        this.setState({
          editKey: setting.key,
          edit,
          editValue,
        });
      };

      editButton = (
        <Button size="sm" variant="info" className="action" disabled={this.state.loading} onClick={edit}>
          <FontAwesomeIcon icon="edit" />
        </Button>
      );

      renderOnChange = null;
    } else {
      editButton = (
        <Button size="sm" variant="info" className="action" disabled={true}>
          <FontAwesomeIcon icon="edit" />
        </Button>
      );

      renderOnChange = value => {
        this.edit(setting.key, {control: setting.control, value});
      };
    }

    let buttons = (
      <ButtonGroup>
        <Button size="sm" variant="danger" className="action" disabled={this.state.loading} onClick={() => this.setState({
          deleteKey: setting.key,
        })}>
          <FontAwesomeIcon icon="trash" />
        </Button>
        {editButton}
      </ButtonGroup>
    );

    let value = null;

    if (isSecret) {
      value = <b title="Secret value, only showed when editing">****</b>;
    } else {
      value = setting.control.render(setting.value, renderOnChange);
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

    if (this.state.editKey === setting.key && this.state.edit) {
      let isValid = this.state.edit.validate(this.state.editValue);

      let save = (e) => {
        e.preventDefault();

        if (isValid) {
          let value = this.state.edit.save(this.state.editValue);
          this.edit(this.state.editKey, value);
        }

        return false;
      };

      let control = this.state.edit.control(isValid, this.state.editValue, editValue => {
        this.setState({editValue});
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
            edit: null,
          })
        }}
      />;
    }

    return (
      <tr key={setting.key}>
        <td className="d-flex">
          <div className="settings-key p-1" lg="3">
            <div className="settings-key-name mb-1">{keyOverride || setting.key}</div>
            <div className="settings-key-doc">{setting.doc}</div>
          </div>

          <div className="flex-grow-1 p-1">{value}</div>
          <div className="p-1">{buttons}</div>
        </td>
      </tr>
    );
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
        let {order, groups, def} = partition(this.state.data);

        content = (
          <div>
            <Table responsive="sm">
              <tbody>
                {def.map(s => this.renderSetting(s))}
              </tbody>
            </Table>
            {order.map(name => {
              let group = groups[name];

              return (
                <Table key={name} responsive="sm">
                  <tbody>
                    <tr>
                      <td className="settings-group">{name}</td>
                    </tr>

                    {group.map(({short, data}) => this.renderSetting(data, short))}
                  </tbody>
                </Table>
              );
            })}
          </div>
        );
      }
    }

    return (
      <div className="settings">
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