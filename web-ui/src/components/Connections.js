import React from "react";
import { RouteLayout } from "./Layout.js";
import { Alert, Table, Button, Form, FormControl, InputGroup, ButtonGroup } from "react-bootstrap";
import { api, currentConnections, currentUser } from "../globals.js";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import Loading from 'shared-ui/components/Loading';
import UserPrompt from "./UserPrompt";
import Connection from "./Connection";

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

    let q = new URLSearchParams(props.location.search);

    this.state = {
      loading: true,
      error: null,
      connections: baseConnections(),
      key: null,
      showKeyCount: null,
      justConnected: q.get("connected"),
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
        outdated: u.outdated,
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

  renderJustConnected() {
    if (!this.state.justConnected) {
      return null;
    }

    let connected = currentConnections.find(c => c.id === this.state.justConnected);

    if (connected === null) {
      return null;
    }

    let otherAccount = null;

    if (currentUser === null) {
      otherAccount = <> (Another Account)</>;
    }

    return (
      <Alert variant="info" className="oxi-center">
        <b>Successfully connected {connected.title}</b>{otherAccount}
      </Alert>
    );
  }

  render() {
    let justConnected = this.renderJustConnected();

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

    let userPrompt = null;

    if (justConnected === null && currentUser === null) {
      userPrompt = <UserPrompt />;
    }

    let content = null;

    if (!this.state.loading) {
      content = <>
        <p>
          Connections allow OxidizeBot to access third party services like Spotify and Twitch. This might be necessary for the bot to provide certain features, like viewer-driven song requests.
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
      </>;
    }

    return (
      <RouteLayout>
        <h2 className="oxi-page-title">My Connections</h2>

        {justConnected}
        {userPrompt}

        <Loading isLoading={this.state.loading} />
        {error}
        {content}
      </RouteLayout>
    );
  }
}