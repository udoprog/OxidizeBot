import {Spinner, True, False, partition} from "../utils";
import React from "react";
import {Alert, Table, Button, InputGroup, Form, Modal} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as ReactMarkdown from 'react-markdown';

/**
 * Special role that everyone belongs to.
 */
const EVERYONE = "@everyone";
const STREAMER = "@streamer";
const MODERATOR = "@moderator";
const SUBSCRIBER = "@subscriber";

/**
 * Check if the given role is a risky role.
 *
 * @param {string} role name of the role to check.
 */
function is_risky_role(role) {
  switch (role) {
    case EVERYONE:
      return true;
    case SUBSCRIBER:
      return true;
    default:
      return false;
  }
}

export default class Authorization extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
      filter: "",
      checked: {
        title: "",
        prompt: "",
        visible: false,
        verify: () => {},
      },
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
    let grants = this.api.authGrants(this.props.current.channel);

    Promise.all([roles, scopes, grants]).then(([roles, scopes, grants]) => {
      let allowsObject = {};

      for (let [scope, role] of grants) {
        allowsObject[`${scope}:${role}`] = true;
      }

      this.setState({
        loading: false,
        error: null,
        data: {roles, scopes, grants: allowsObject},
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
    this.api.authDeleteGrant(scope, role)
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
    this.api.authInsertGrant({scope, role})
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

  /**
   * Render authentication button.
   */
  renderAuthButton(scope, role, grants) {
    let has_implicit = null;
    let title = null;

    let is_allowed = role => grants[`${scope.scope}:${role}`] || false;

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
        has_implicit = test_implicit([EVERYONE, SUBSCRIBER]) || false;
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

        if (is_risky_role(role.role) && scope.risk === "high") {
          allow = () => {
            this.setState({
              checked: {
                title: "Grant high-risk scope?",
                prompt: (
                  <div>
                    <div><b>{scope.scope}</b> is a <b>high risk</b> scope.</div>
                    <div className="mb-3">Granting it to <b>{role.role}</b> might pose a <b>security risk</b>.</div>
                    <div className="align-center">
                      <em>Are you sure?</em>
                    </div>
                  </div>
                ),
                visible: true,
                verify: () => this.allow(scope.scope, role.role),
              }
            });
          };
        }

        button = (
          <Button className="auth-boolean-icon" title={title} size="sm" variant="danger" onClick={allow}>
            <False />
          </Button>
        );
      }
    }

    return <td key={role.role} align="center">{button}</td>;
  }

  /**
   * Render a single group body.
   */
  renderScope(scope, data, nameOverride = null) {
    return (
      <tr key={scope.scope}>
        <td className="auth-scope-key">
          <div className="auth-scope-key-name">{nameOverride || scope.scope}</div>
          <div className="auth-scope-key-doc">
            <ReactMarkdown source={scope.doc} />
          </div>
        </td>
        {data.roles.map(role => this.renderAuthButton(scope, role, data.grants))}
      </tr>
    );
  }

  /**
   * Render a single group.
   */
  renderGroup(group, name, data) {
    return [
      <tr key={`title:${name}`} className="auth-scope-short">
        <td colSpan={data.roles.length + 1}>{name}</td>
      </tr>,
      group.map(d => {
        return this.renderScope(d.data, data, d.short);
      }),
    ];
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

    let data = null;

    if (this.state.data) {
      data = this.filtered(this.state.data);
    }

    if (data && data.scopes.length > 0) {
      let {order, groups, def} = partition(data.scopes, s => s.scope);

      content = (
        <Table key={name} className="mb-0">
          <tbody>
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
            {def.map(scope => this.renderScope(scope, data))}
            {order.map(name => this.renderGroup(groups[name], name, data))}
          </tbody>
        </Table>
      );
    } else {
      content = (
        <Alert variant="info">
          No Scopes!
        </Alert>
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

    let handleClose = () => {
      this.setState({
        checked: {
          title: "",
          prompt: "",
          visible: false,
          verify: () => {},
        }
      });
    };

    let handleVerify = () => {
      this.state.checked.verify();
      handleClose();
    };

    let modal = (
      <Modal show={!!this.state.checked.visible} onHide={handleClose}>
        <Modal.Header closeButton>
          <Modal.Title className="align-center">{this.state.checked.title}</Modal.Title>
        </Modal.Header>
        <Modal.Body className="align-center">{this.state.checked.prompt}</Modal.Body>
        <Modal.Footer>
          <Button variant="secondary" onClick={handleClose}>
            No
          </Button>
          <Button variant="primary" onClick={handleVerify}>
            Yes
          </Button>
        </Modal.Footer>
      </Modal>
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
        {modal}
      </div>
    );
  }
}