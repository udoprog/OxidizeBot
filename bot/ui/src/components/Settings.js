import React from "react";
import {Spinner, partition} from "../utils";
import {Form, Button, Alert, Table, InputGroup} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as types from "./Settings/Types.js";
import Setting from "./Setting";

export default class Settings extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
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

    let params = {
      keyFilter: this.props.keyFilter
    };

    this.api.settings(params)
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
    });

    this.api.deleteSetting(key)
      .then(
        () => this.list(),
        e => {
          this.setState({
            loading: false,
            error: `failed to delete setting: ${e}`,
          });
        }
      );
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
      };
    });

    this.api.editSetting(key, control.serialize(value))
      .then(
        () => this.list(),
        e => {
          this.setState({
            loading: false,
            error: `failed to edit setting: ${e}`,
          });
        }
      );
  }

  /**
   * Filter the data if applicable.
   */
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

  /**
   * Render the given name as a set of clickable links.
   */
  filterLinks(name) {
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

    let loading = null;

    if (this.state.loading) {
      loading = <Spinner />;
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
                {def.map(s => {
                  return <Setting
                    key={s.key}
                    setting={s}
                    onEdit={this.edit.bind(this)}
                    onDelete={this.delete.bind(this)} />;
                })}
              </tbody>
            </Table>

            {order.map(name => {
              let group = groups[name];
              let title = null;

              if (this.props.filterable) {
                title = this.filterLinks(name);
              } else {
                title = name;
              }

              return (
                <Table className="mb-0" key={name}>
                  <tbody>
                    <tr>
                      <th className="settings-group">{title}</th>
                    </tr>

                    {group.map(({short, data}) => {
                      return <Setting
                        key={data.key}
                        setting={data}
                        onEdit={this.edit.bind(this)}
                        onDelete={this.delete.bind(this)}
                        keyOverride={short} />;
                    })}
                  </tbody>
                </Table>
              );
            })}
          </div>
        );
      }
    }

    let filter = null;

    if (this.props.filterable) {
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

      filter = (
        <Form className="mt-4 mb-4">
          <InputGroup>
            <Form.Control value={this.state.filter} placeholder="Filter Settings" onChange={filterOnChange}></Form.Control>
            {clear}
          </InputGroup>
        </Form>
      );
    }

    return (
      <div className="settings">
        {error}
        {filter}
        {content}
        {loading}
      </div>
    );
  }
}