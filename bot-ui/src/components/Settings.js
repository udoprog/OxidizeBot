import React from "react";
import {partition} from "../utils";
import {Form, Button, Alert, Table, InputGroup} from "react-bootstrap";
import * as types from "./Settings/Types.js";
import Setting from "./Setting";
import {Loading, Error} from 'shared-ui/components';

export default class Settings extends React.Component {
  constructor(props) {
    super(props);

    let filter = "";

    if (this.props.location) {
      let search = new URLSearchParams(this.props.location.search);
      filter = search.get("q") || "";
    }

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
      // current filter being applied to filter visible settings.
      filter,
    };
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list()
  }

  /**
   * Update the current filter.
   */
  setFilter(filter) {
    if (this.props.location) {
      let path = `${this.props.location.pathname}`;

      if (!!filter) {
        let search = new URLSearchParams(this.props.location.search);
        search.set("q", filter);
        path = `${path}?${search}`
      }

      this.props.history.replace(path);
    }

    this.setState({filter});
  }

  /**
   * Refresh the list of settings.
   */
  list() {
    this.setState({
      loading: true,
    });

    this.api.settings(this.props.filter)
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
            ...d.schema,
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
          error: `failed to request settings: ${e}`,
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
      return parts.every(p => {
        if (d.key.toLowerCase().indexOf(p) != -1) {
          return true;
        }

        if (d.title && d.title.toLowerCase().indexOf(p) != -1) {
          return true;
        }

        return false;
      });
    });
  }

  /**
   * Render the given name as a set of clickable links.
   */
  filterLinks(name) {
    let setFilter = filter => () => this.setFilter(`^${filter}/`);

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

  content() {
    if (!this.state.data) {
      return null;
    }

    if (this.state.data.length === 0) {
      return (
        <Alert variant="info">No Settings!</Alert>
      );
    }

    let settingProps = {
      useTitle: !!this.props.useTitle,
      disableDoc: !!this.props.disableDoc,
    };

    let data = this.filtered(this.state.data);

    if (!this.props.group) {
      return (
        <div>
          <Table className="mb-0">
            <tbody>
              {data.map(s => {
                return <Setting
                  key={s.key}
                  setting={s}
                  onEdit={this.edit.bind(this)}
                  onDelete={this.delete.bind(this)}
                  {...settingProps} />;
              })}
            </tbody>
          </Table>
        </div>
      );
    }

    let {order, groups, def} = partition(data, d => d.key);

    return (
      <div>
        <Table className="mb-0">
          <tbody>
            {def.map(s => {
              return <Setting
                key={s.key}
                setting={s}
                onEdit={this.edit.bind(this)}
                onDelete={this.delete.bind(this)}
                {...settingProps} />;
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
                    keyOverride={short}
                    {...settingProps} />;
                })}
              </tbody>
            </Table>
          );
        })}
      </div>
    );
  }

  render() {
    let content = this.content();
    let filter = null;

    if (this.props.filterable) {
      let filterOnChange = e => this.setFilter(e.target.value);
      let clearFilter = () => this.setFilter("");

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
            <Form.Control value={this.state.filter} placeholder="Search" onChange={filterOnChange}></Form.Control>
            {clear}
          </InputGroup>
        </Form>
      );
    }

    return (
      <div className="settings">
        <Loading isLoading={this.state.loading} />
        <Error error={this.state.error} />
        {filter}
        {content}
      </div>
    );
  }
}