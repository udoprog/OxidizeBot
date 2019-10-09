import React from "react";
import {Form, Button, Alert, Table, InputGroup, Modal, ButtonGroup} from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import * as moment from "moment";
import {Loading, Error} from 'shared-ui/components';

export default class Cache extends React.Component {
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
      show: null,
    };
  }

  async componentDidMount() {
    this.interval = setInterval(() => this.setState({ time: Date.now() }), 1000);
    await this.list();
  }

  componentWillUnmount() {
    clearInterval(this.interval);
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
  async list() {
    this.setState({loading: true});

    try {
      let data = await this.api.cache();

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to request cache: ${e}`,
        data: null,
      });
    }
  }

  /**
   * Remove a cache entry.
   */
  async cacheDelete(key) {
    this.setState({loading: true});

    try {
      await this.api.cacheDelete(key);
      await this.list();
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to delete cache entry: ${e}`
      });
    }
  }

  /**
   * Filter the data if applicable.
   */
  filtered(data) {
    if (!this.state.filter) {
      return data;
    }

    let parts = this.state.filter.split(" ").map(f => f.toLowerCase());

    return data.filter(d => {
      return parts.every(p => {
        let [ns, key] = d.key;

        if (ns !== null && ns.toLowerCase().indexOf(p) != -1) {
          return true;
        }

        if (typeof key !== "object") {
          return false;
        }

        let any = false;

        for (let keyName in key) {
          let v = key[keyName];

          if (typeof v === "string") {
            any = v.toLowerCase().indexOf(p) != -1;

            if (any) {
              break;
            }
          }
        }

        return any;
      });
    });
  }

  modal(now) {
    let header = null;
    let body = null;

    if (this.state.show !== null) {
      let {key, value, expires_at} = this.state.show;
      let [ns, k] = key;

      if (ns !== null) {
        ns = <span><b>{ns}</b> &nbsp;</span>;
      }

      header = (
        <span>{ns} <code>{JSON.stringify(k)}</code> {this.renderExpiresAt(now, expires_at)}</span>
      );
      body = <code><pre>{JSON.stringify(value, null, 2)}</pre></code>;
    }

    let hide = () => {
      this.setState({
        show: null,
      });
    };

    return (
      <Modal className="chat-settings" show={this.state.show !== null} onHide={hide}>
        <Modal.Header>{header}</Modal.Header>
        <Modal.Body>{body}</Modal.Body>
      </Modal>
    );
  }

  groupByNamespace(data) {
    let def = [];
    let groups = {};

    for (let d of data) {
      let {key, value} = d;
      let [ns, k] = key;

      if (ns === null) {
        def.push({key: k, data: d});
        continue;
      }

      let group = groups[ns];

      if (!group) {
        groups[ns] = [{key: k, data: d}];
        continue;
      }

      group.push({key: k, data: d});
    }

    let order = Object.keys(groups);
    order.sort();

    return {def, groups, order};
  }

  /**
   * Render when a thing expires.
   */
  renderExpiresAt(now, at) {
    let when = moment(at);
    let diff = moment(when - now);
    return diff.format("D.hh:mm:ss");
  }

  /**
   * Render a single key.
   */
  renderKey(now, i, key, data) {
    let cacheDelete = () => this.cacheDelete(data.key);
    let show = () => this.setState({show: data});

    return (
      <tr key={i}>
        <td>
          <code>{JSON.stringify(key)}</code>
        </td>
        <td className="cache-expires">
          {this.renderExpiresAt(now, data.expires_at)}
        </td>
        <td width="1%">
          <ButtonGroup>
            <Button variant="danger" onClick={cacheDelete}><FontAwesomeIcon icon="trash" /></Button>
            <Button onClick={show}><FontAwesomeIcon icon="eye" /></Button>
          </ButtonGroup>
        </td>
      </tr>
    );
  }

  render() {
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

    let filter = (
      <Form className="mt-4 mb-4">
        <InputGroup>
          <Form.Control value={this.state.filter} placeholder="Search" onChange={filterOnChange}></Form.Control>
          {clear}
        </InputGroup>
      </Form>
    );

    let now = moment();

    let modal = this.modal(now);

    let content = null;

    if (this.state.data !== null) {
      let data = this.filtered(this.state.data);
      let {def, groups, order} = this.groupByNamespace(data);

      content = (
        <Table>
          <thead>
            <tr>
              <th>key</th>
              <th>expires</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {def.map(({key, data}, i) => this.renderKey(now, i, key, data))}
          </tbody>
          {order.map(o => {
            let title = (
              <tbody key="title">
                <tr>
                  <td className="cache-namespace-header">{o}</td>
                </tr>
              </tbody>
            );

            let body = (
              <tbody key="body">
                {groups[o].map(({key, data}, i) => this.renderKey(now, i, key, data))}
              </tbody>
            );

            return [title, body];
          })}
        </Table>
      );
    }

    return (
      <div className="cache">
        <Loading isLoading={this.state.loading} />
        <Error error={this.state.error} />
        {filter}
        {modal}
        {content}
      </div>
    );
  }
}