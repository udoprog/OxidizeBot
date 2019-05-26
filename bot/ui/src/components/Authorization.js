import {Spinner, True, False} from "../utils";
import React from "react";
import {Alert, Table, Button, InputGroup, Form} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";

/**
 * Special role that everyone belongs to.
 */
const EVERYONE = "@everyone";
const STREAMER = "@streamer";
const MODERATOR = "@moderator";
const SUBSCRIBER = "@subscriber";

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
              <th className="table-fill"></th>
              {data.roles.map(role => {
                return (
                  <th key={role.role} title={role.doc}>
                    <div className="auth-role-name">{role.role}</div>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {data.scopes.map(scope => {
              return (
                <tr key={scope.scope}>
                  <td className="auth-scope-key">
                    <div className="auth-scope-key-name">{scope.scope}</div>
                    <div className="auth-scope-key-doc">{scope.doc}</div>
                  </td>
                  {data.roles.map(role => {
                    let has_implicit = null;
                    let title = null;

                    let is_allowed = role => data.allows[`${scope.scope}:${role}`] || false;

                    let test_implicit = roles => {
                      for (let role of roles) {
                        if (is_allowed(role)) {
                          return role;
                        }
                      }

                      return null;
                    }

                    switch (role.role) {
                      case EVERYONE:
                        break;
                      case STREAMER:
                        has_implicit = test_implicit([EVERYONE, MODERATOR, SUBSCRIBER]) || false;
                        break;
                      default:
                        has_implicit = test_implicit([EVERYONE]) || false;
                        break;
                    }

                    let allowed = !!has_implicit || is_allowed(role.role) || false;
                    let button = null;

                    if (!!has_implicit) {
                      title = `allowed because ${has_implicit} is allowed`;
                    } else {
                      if (allowed) {
                        title = `${scope.scope} scope is allowed by ${role.role}`;
                      } else {
                        title = `${scope.scope} scope is denied to ${role.role}`;
                      }
                    }

                    if (!!has_implicit) {
                      button = (
                        <Button className="auth-boolean-icon" disabled={true} title={title} size="sm" variant="secondary">
                          <True />
                        </Button>
                      );
                    } else {
                      if (allowed) {
                        let deny = () => this.deny(scope.scope, role.role);

                        button = (
                          <Button className="auth-boolean-icon" title={title} size="sm" variant="success" onClick={deny}>
                            <True />
                          </Button>
                        );
                      } else {
                        let allow = () => this.allow(scope.scope, role.role);

                        button = (
                          <Button className="auth-boolean-icon" title={title} size="sm" variant="danger" onClick={allow}>
                            <False />
                          </Button>
                        );
                      }
                    }

                    return (
                      <td align="center" key={role.role}>{button}</td>
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