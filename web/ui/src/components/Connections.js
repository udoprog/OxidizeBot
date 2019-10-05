import React from "react";
import { RouteLayout } from "./Layout.js";
import { Alert, Table, Button, Form, FormControl, InputGroup, ButtonGroup } from "react-bootstrap";
import { api, currentConnections, currentUser } from "../globals.js";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import copy from 'copy-to-clipboard';
import Loading from "./Loading.js";
import If from "./If.js";
import UserPrompt from "./UserPrompt";

class CountDown {
  constructor(count, call, end) {
    this.count = count;

    var self = this;

    this.interval = setInterval(() => {
      if (self.count <= 1) {
        self.stop();
        end();
      } else {
        self.count -= 1;
        call(self.count);
      }
    }, 1000);

    call(self.count);
  }

  stop() {
    if (this.interval !== null) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }
}

class Connection extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      copied: false,
    };

    this.clearCopied = null;
  }

  async connect() {
    try {
      let result = await api.connectionsCreate(this.props.id);
      location.href = result.auth_url;
    } catch(e) {
      this.props.onError(e);
    }
  }

  async copy() {
    if (this.clearCopied !== null) {
      clearTimeout(this.clearCopied);
      this.clearCopied = null;
    }

    let result = await api.connectionsCreate(this.props.id);
    copy(result.auth_url);
    this.setState({copied: true});
    this.clearCopied = setTimeout(() => this.setState({copied: false}), 2000);
  }

  async disconnect() {
    let result = await api.connectionsRemove(this.props.id);

    if (this.props.onDisconnect) {
      this.props.onDisconnect();
    }
  }

  icon() {
    switch (this.props.type) {
      case "twitch":
        return <FontAwesomeIcon icon={['fab', 'twitch']} />;
      case "youtube":
        return <FontAwesomeIcon icon={['fab', 'youtube']} />;
      case "spotify":
        return <FontAwesomeIcon icon={['fab', 'spotify']} />;
      default:
        return <FontAwesomeIcon icon="globe" />;
    }
  }

  meta() {
    let account = null;

    switch (this.props.type) {
      case "twitch":
        if (!this.props.meta || !this.props.meta.login) {
          return null;
        }

        account = <a href={`https://twitch.tv/${this.props.meta.login}`}><b>{this.props.meta.login}</b></a>;
        break;
      case "spotify":
        if (!this.props.meta || !this.props.meta.display_name) {
          return null;
        }

        let product = null;

        if (this.props.meta.product) {
          product = <> ({this.props.meta.product})</>;
        }

        account = <><b>{this.props.meta.display_name}</b>{product}</>;

        if (this.props.meta.external_urls && this.props.meta.external_urls.spotify) {
          account = <a href={this.props.meta.external_urls.spotify}>{account}</a>;
        }

        break;
      default:
        return null;
    }

    return <div className="connected-meta">Connected account: {account}</div>;
  }

  validate() {
    switch (this.props.type) {
      case "spotify":
        if (!this.props.meta || !this.props.meta.product) {
          return null;
        }

        if (this.props.meta.product === "premium") {
          return null;
        }

        return <div className="connected-validate danger"><b>You need a Premium Spotify Account</b></div>;
      default:
        return null;
    }
  }

  render() {
    let icon = this.icon();
    let button = null;

    if (this.props.connected !== null) {
      if (this.props.connected) {
        button = <Button size="sm" variant="danger" onClick={() => this.disconnect()} title="Remove connection">Remove&nbsp;<FontAwesomeIcon icon="trash" /></Button>;
      } else {
        let copyButton = null;

        if (this.state.copied) {
          copyButton = (
            <Button size="sm" variant="success" disabled={true}>
              <FontAwesomeIcon icon="check" />
            </Button>
          );
        } else {
          copyButton = (
            <Button disabled={currentUser === null} size="sm" variant="success" onClick={() => this.copy()} title="Copy to clipboard">
              <FontAwesomeIcon icon="copy" />
            </Button>
          );
        }

        button = (
          <ButtonGroup>
            <Button disabled={currentUser === null} size="sm" variant="primary" onClick={() => this.connect()}>Connect</Button>
            {copyButton}
          </ButtonGroup>
        );
      }
    }

    let meta = this.meta();
    let validate = this.validate();

    return (
      <tr>
        <td className="connected">
          <div className="connected-title">{icon} {this.props.title}</div>
          {meta}
          {validate}
          <div className="connected-description">{this.props.description}</div>
        </td>
        <td align="right">{button}</td>
      </tr>
    );
  }
}

function baseConnections() {
  let connections = {};

  for (let c of currentConnections) {
    connections[c.id] = null;
  }

  return connections;
}

export default class Connections extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      loading: true,
      error: null,
      connections: baseConnections(),
      key: null,
      showKeyCount: null,
    };
    this.showKey = null;
  }

  async componentDidMount() {
    // refresh list of connections if we are logged in.
    if (currentUser !== null) {
      try {
        await this.refreshConnections();
      } catch(e) {
        this.setState({error: e});
      }
    }

    this.setState({loading: false});
  }

  async refreshConnections() {
    let [update, key] = await Promise.all([api.connectionsList(), api.getKey()]);

    let connections = {};

    for (let c of currentConnections) {
      connections[c.id] = {
        connected: false,
      };
    }

    for (var u of update) {
      connections[u.id] = {
        meta: u.meta,
        connected: true,
      };
    }

    this.setState({connections, key: key.key});
  }

  async onDisconnect() {
    this.setState({error: null});

    try {
      await this.refreshConnections();
    } catch(e) {
      this.onError(e);
    }
  }

  async generateKey() {
    this.setState({error: null});

    try {
      let key = await api.createKey();
      this.setState({key: key.key});
    } catch(e) {
      this.setState({error: e});
      return;
    }
  }

  async clearKey() {
    this.setState({error: null});

    try {
      await api.deleteKey();
      this.setState({key: null});
      this.hideKey();
    } catch(e) {
      this.setState({error: e});
      return;
    }
  }

  onError(e) {
    this.setState({error: e});
  }

  send() {
    let query = "";

    if (this.state.key) {
      query = `?key=${encodeURIComponent(this.state.key)}`;
    }

    location.href = `http://localhost:12345/api/auth/key${query}`;
  }

  hideKey() {
    if (this.showKey !== null) {
      this.showKey.stop();
      this.showKey = null;
    }

    this.setState({showKeyCount: null});
  }

  showKeyFor(count) {
    if (this.showKey !== null) {
      this.showKey.stop();
      this.showKey = null;
    }

    this.showKey = new CountDown(count, (i) => {
      this.setState({showKeyCount: i});
    }, () => {
      this.setState({showKeyCount: null});
    });
  }

  render() {
    let error = null;

    if (this.state.error !== null) {
      error = (
        <Alert variant="danger">{this.state.error.toString()}</Alert>
      );
    }

    let showKey = null;

    if (this.state.key !== null) {
      if (this.state.showKeyCount !== null) {
        showKey = <Button variant="light" onClick={() => this.hideKey()}>{this.state.showKeyCount} <FontAwesomeIcon icon="eye-slash" title="Hide key" /></Button>
      } else {
        showKey = <Button variant="light" onClick={() => this.showKeyFor(10)} title="Click to show secret key for 10 seconds"><FontAwesomeIcon icon="eye" /></Button>;
      }
    }

    let value = "";
    let placeholder = null;
    let clear = null;
    let generate = null;
    let send;

    if (this.state.showKeyCount !== null && this.state.key != null) {
      value = this.state.key;
    }

    if (this.state.key === null) {
      placeholder = "no key available";

      generate = (
        <Button disabled={currentUser === null} variant="primary" onClick={() => this.generateKey()} title="Generate a new secret key.">
          Generate
        </Button>
      );
    } else {
      placeholder = "key hidden";

      clear = (
        <Button variant="danger" disabled={this.state.key === null} onClick={() => this.clearKey()} title="Clear the current key without regenerating it.">
          Clear
        </Button>
      );

      generate = (
        <Button variant="primary" onClick={() => this.generateKey()} title="Create a new key, invalidating the existing key.">Regenerate</Button>
      );

      send = (
        <Button variant="info" title="Send key to bot" onClick={() => this.send()}><FontAwesomeIcon icon="share" /></Button>
      );
    }

    let key = (
      <Form className="mb-3">
        <InputGroup>
          <FormControl readOnly={true} value={value} placeholder={placeholder} />
          <InputGroup.Append>
            {showKey}
            {clear}
            {generate}
            {send}
          </InputGroup.Append>
        </InputGroup>
      </Form>
    );

    let userPrompt;

    if (currentUser === null) {
      userPrompt = <UserPrompt />;
    }

    return (
      <RouteLayout>
        <h2 className="page-title">My Connections</h2>

        {userPrompt}

        <Loading isLoading={this.state.loading} />
        {error}

        <If isNot={this.state.loading}>
          <p>
            Connections allows OxidizeBot to access the services associated with the granted connection. This might be necessary for the bot to provide certain services, like song requests through YouTube.
          </p>

          <h4>Secret Key</h4>

          <p>
            This key should be configured in your bot to allow it to communicate with this service.
          </p>

          {key}

          <h4>Connections</h4>

          <p>
            Each connection adds capabilities to OxidizeBot.
            You'll have to enable and authenticate them here.
          </p>
          <Table>
            <tbody>
              {currentConnections.map((c, index) => {
                return <Connection
                  key={index}
                  onDisconnect={() => this.onDisconnect(c.id)}
                  onError={e => this.onError(e)}
                  {...c} {...this.state.connections[c.id]} />;
              })}
            </tbody>
          </Table>
        </If>
      </RouteLayout>
    );
  }
}