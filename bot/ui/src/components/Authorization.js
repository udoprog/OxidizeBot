import {Spinner, True, False} from "../utils";
import React from "react";
import {Alert, Table, Button, InputGroup, Form} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";

export default class Authorization extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
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
   * Refresh the list of after streams.
   */
  list() {
    this.setState({
      loading: true,
    });

    let roles = this.api.authRoles(this.props.current.channel);
    let scopes = this.api.authScopes(this.props.current.channel);
    let allows = this.api.authAllows(this.props.current.channel);

    Promise.all([roles, scopes, allows]).then(([roles, scopes, allows]) => {
      let allowsObject = {};

      for (let [scope, role] of allows) {
        allowsObject[`${scope}:${role}`] = true;
      }

      this.setState({
        loading: false,
        error: null,
        data: {roles, scopes, allows: allowsObject},
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

  deny(scope, role) {
    this.api.authDeleteAllow(scope, role)
      .then(() => {
        return this.list();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to insert an allow permit: ${e}`,
        });
      });
  }

  allow(scope, role) {
    this.api.authInsertAllow({scope, role})
      .then(() => {
        return this.list();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to insert an allow permit: ${e}`,
        });
      });
  }

  filtered(data) {
    if (!this.state.filter) {
      return data;
    }

    let parts = this.state.filter.split(" ").map(f => f.toLowerCase());

    let scopes = data.scopes.filter(scope => {
      return parts.every(p => {
        if (scope.scope.toLowerCase().indexOf(p) != -1) {
          return true;
        }

        return scope.doc.toLowerCase().indexOf(p) != -1;
      });
    });

    return Object.assign({}, data, {scopes});
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
      let data = this.filtered(this.state.data);

      content = (
        <Table responsive="sm">
          <thead>
            <tr>
              <th width="20%"></th>
              {data.roles.map(role => {
                return <th key={role}>{role}</th>;
              })}
            </tr>
          </thead>
          <tbody>
            {data.scopes.map(scope => {
              return (
                <tr key={scope.scope}>
                  <td className="scope-key">
                    <div className="scope-key-name">{scope.scope}</div>
                    <div className="scope-key-doc">{scope.doc}</div>
                  </td>
                  {data.roles.map(role => {
                    let key = `${scope.scope}:${role}`;
                    let checked = data.allows[key] || false;

                    let allow = () => this.allow(scope.scope, role);
                    let deny = () => this.deny(scope.scope, role);

                    let button = null;

                    if (checked) {
                      button = <Button title="Deny scope" size="sm" variant="success" onClick={deny}><True /></Button>;
                    } else {
                      button = <Button title="Allow scope" size="sm" variant="danger" onClick={allow}><False /></Button>;
                    }

                    return (
                      <td key={role}>{button}</td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </Table>
      );
    }

    let clear = null;

    if (!!this.state.filter) {
      let clearFilter = () => {
        this.setState({filter: ""});
      };

      clear = (
        <InputGroup.Append>
          <Button variant="primary" onClick={clearFilter}>Clear Filter</Button>
        </InputGroup.Append>
      );
    }

    let filterOnChange = e => {
      this.setState({filter: e.target.value});
    };

    let filter = (
      <Form className="mt-4 mb-4">
        <InputGroup>
          <Form.Control value={this.state.filter} placeholder="Filter Scopes" onChange={filterOnChange}></Form.Control>
          {clear}
        </InputGroup>
      </Form>
    );

    return (
      <div>
        <h2>
          Authorization
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