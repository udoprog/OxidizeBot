import React from "react";
import {Spinner, partition} from "../utils";
import {Form, Button, Alert, Table, ButtonGroup, InputGroup, Row, Col} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as types from "./Settings/Types.js";
import * as ReactMarkdown from 'react-markdown';

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
      // current filter being applied to filter visible settings.
      filter: "",
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
          let control = types.decode(d.schema.type);

          let value = null;

          if (d.value !== null) {
            value = control.construct(d.value);
          }

          return {
            key: d.key,
            control,
            value,
            doc: d.schema.doc,
            secret: d.schema.secret,
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
  edit(key, control, value) {
    this.setState(state => {
      let data = state.data.map(setting => {
        if (setting.key === key) {
          return Object.assign(setting, {value});
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

  filtered(data) {
    if (!this.state.filter) {
      return data;
    }

    if (this.state.filter.startsWith('^')) {
      let filter = this.state.filter.substring(1);
      return data.filter(d => d.key.startsWith(filter));
    }

    let parts = this.state.filter.split(" ").map(f => f.toLowerCase());

    return data.filter(d => {
      return parts.every(p => d.key.toLowerCase().indexOf(p) != -1);
    });
  }

  renderSetting(setting, keyOverride = null) {
    // onChange handler used for things which support immediate editing.
    let renderOnChange = null;

    let buttons = [];
    let isDeleting = this.state.deleteKey === setting.key;
    let isEditing = this.state.editKey === setting.key;
    let isSecret = setting.key.startsWith(SECRET_PREFIX) || setting.secret;

    if (setting.control.optional) {
      let del = () => {
        this.setState({
          deleteKey: setting.key,
        });
      };

      if (setting.value !== null) {
        buttons.push(
          <Button key="delete" size="sm" variant="danger" className="action" disabled={this.state.loading} onClick={del}>
            <FontAwesomeIcon icon="trash" />
          </Button>
        );
      }
    }

    if (setting.control.hasEditControl()) {
      let edit = () => {
        let value = setting.value;

        if (value == null) {
          value = setting.control.default();
        }

        let edit = setting.control.editControl();
        let editValue = setting.control.edit(value);

        this.setState({
          editKey: setting.key,
          edit,
          editValue,
        });
      };

      buttons.push(
        <Button key="edit" size="sm" variant="info" className="action" disabled={this.state.loading} onClick={edit}>
          <FontAwesomeIcon icon="edit" />
        </Button>
      );

      renderOnChange = null;
    } else {
      renderOnChange = value => {
        this.edit(setting.key, setting.control, value);
      };
    }

    if (buttons.length > 0) {
      buttons = (
        <div className="ml-3">
          <ButtonGroup>{buttons}</ButtonGroup>
        </div>
      );
    }

    let value = null;

    if (setting.value === null) {
      value = <em title="Value not set">not set</em>;;
    } else {
      if (isSecret) {
        value = <b title="Secret value, only showed when editing">****</b>;
      } else {
        value = setting.control.render(setting.value, renderOnChange);
      }
    }

    if (isDeleting) {
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

    if (isEditing && this.state.edit) {
      let isValid = this.state.edit.validate(this.state.editValue);

      let save = (e) => {
        e.preventDefault();

        if (isValid) {
          let value = this.state.edit.save(this.state.editValue);
          this.edit(this.state.editKey, setting.control, value);
        }

        return false;
      };

      let control = this.state.edit.render(isValid, this.state.editValue, editValue => {
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
        <td>
          <Row>
            <Col lg="3" className="settings-key mb-1">
              <div className="settings-key-name mb-1">{keyOverride || setting.key}</div>
              <div className="settings-key-doc">
                <ReactMarkdown source={setting.doc} />
              </div>
            </Col>

            <Col lg="9">
              <div className="d-flex align-items-top">
                <div className="flex-fill align-middle">{value}</div>
                {buttons}
              </div>
            </Col>
          </Row>
        </td>
      </tr>
    );
  }

  nameLinks(name) {
    let setFilter = filter => () => {
      this.setState({filter: `^${filter}/`});
    };

    let parts = name.split("/");
    let path = [];
    let len = 0;
    let out = [];

    for (let p of parts) {
      path.push(p);
      len += p.length;
      let filter = name.substring(0, Math.min(len, name.length));
      len += 1;

      out.push(
        <a
          className="settings-filter"
          title={`Filter for "${filter}" prefix.`}
          key={filter}
          onClick={setFilter(filter)}>{p}</a>
      );

      out.push("/");
    }

    return out;
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
        let {order, groups, def} = partition(this.filtered(this.state.data), d => d.key);

        content = (
          <div>
            <Table className="mb-0">
              <tbody>
                {def.map(s => this.renderSetting(s))}
              </tbody>
            </Table>

            {order.map(name => {
              let group = groups[name];
              let nameLinks = this.nameLinks(name);

              return (
                <Table className="mb-0" key={name}>
                  <tbody>
                    <tr>
                      <th className="settings-group">
                        {nameLinks}
                      </th>
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

    let filterOnChange = e => {
      this.setState({filter: e.target.value});
    };

    let clearFilter = () => {
      this.setState({filter: ""});
    };

    let clear = null;

    if (!!this.state.filter) {
      clear = (
        <InputGroup.Append>
          <Button variant="primary" onClick={clearFilter}>Clear Filter</Button>
        </InputGroup.Append>
      );
    }

    let filter = (
      <Form className="mt-4 mb-4">
        <InputGroup>
          <Form.Control value={this.state.filter} placeholder="Filter Settings" onChange={filterOnChange}></Form.Control>
          {clear}
        </InputGroup>
      </Form>
    );

    return (
      <div className="settings">
        <h2>
          Settings
          {refresh}
        </h2>
        {error}
        {filter}
        {content}
        {loading}
      </div>
    );
  }
}