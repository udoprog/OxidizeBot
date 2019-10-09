import React from "react";
import {Button, Form, Alert} from "react-bootstrap";
import * as utils from "../utils.js";
import {Nav, Row, Col, Card} from "react-bootstrap";
import {Route, Link} from "react-router-dom";
import Loading from 'shared-ui/components/Loading';

export default class ImportExport extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let path = this.props.location.pathname;

    return (
      <Row>
        <Col sm="2">
          <Nav className="flex-column" variant="pills">
            <Nav.Link as={Link} active={path === "/import-export/phantombot"} to="/import-export/phantombot">
              PhantomBot
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/import-export/drangrybot"} to="/import-export/drangrybot">
              DrangryBot
            </Nav.Link>
          </Nav>
        </Col>
        <Col>
          <Route path="/import-export" exact render={props => <Index {...props} />} />
          <Route path="/import-export/phantombot" render={props => <PhantomBot api={this.props.api} {...props} />} />
          <Route path="/import-export/drangrybot" render={props => <DrangryBot api={this.props.api} {...props} />} />
        </Col>
      </Row>
    );
  }
}

class Index extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <div>
        <h2>Import / Export modules for OxidizeBot</h2>

        <p>
          In here you'll find modules for importing and exporting data to third party systems.
        </p>
      </div>
    );
  }
}

class PhantomBot extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
    };
  }

  async exportCsv(e) {
    e.preventDefault();

    let balances = await this.api.exportBalances();
    var balancesCsv = "";

    for (var balance of balances) {
      balancesCsv += `,${balance.user},${balance.amount}\r\n`;
    }

    utils.download("text/plain", balancesCsv, "balances.csv");
    return false;
  }

  render() {
    return (
      <>
        <div className="mb-3">
          <h2>PhantomBot</h2>

          <p>
            Site: <a href="https://phantombot.tv">phantombot.tv</a>
          </p>

          <Card>
            <Card.Body>
              <blockquote className="blockquote mb-0">
                PhantomBot is an actively developed open source interactive Twitch bot with a vibrant community that provides entertainment and moderation for your channel, allowing you to focus on what matters the most to you - your game and your viewers.
              </blockquote>
            </Card.Body>
          </Card>
        </div>

        <Row>
          <Col>
            <h4>Import</h4>
            <PhantomBotImportCsvForm api={this.api} />
          </Col>

          <Col>
            <h4>Export</h4>

            <Form onSubmit={e => this.exportCsv(e)}>
              <Button type="submit">Export to File</Button>
            </Form>
          </Col>
        </Row>
      </>
    );
  }
}


class DrangryBot extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
    };
  }

  async exportScv(e) {
    e.preventDefault();

    let balances = await this.api.exportBalances();
    var balancesCsv = "Name,Balance,TimeInSeconds\r\n";

    for (var balance of balances) {
      balancesCsv += `${balance.user},${balance.amount},${balance.watch_time}\r\n`;
    }

    utils.download("text/plain", balancesCsv, "balances.csv");
    return false;
  }

  render() {
    return (
      <>
        <div className="mb-3">
          <h2>DrangryBot</h2>

          <p>
            Site: <a href="https://drangrybot.tv">drangrybot.tv</a>
          </p>

          <Card>
            <Card.Body>
              <blockquote className="blockquote mb-0">
                It is your all-in-one solution to enhance your Twitch channel.
              </blockquote>
            </Card.Body>
          </Card>
        </div>

        <Row>
          <Col>
            <h4>Import</h4>
            <DrangryBotImportCsvForm api={this.api} />
          </Col>

          <Col>
            <h4>Export</h4>

            <Form onSubmit={e => this.exportScv(e)}>
              <Button type="submit">Export to File</Button>
            </Form>
          </Col>
        </Row>
      </>
    );
  }
}

class DrangryBotImportCsvForm extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      success: null,
      error: null,
      channel: "",
      text: "Name,Balance,TimeInSeconds\r\nsetbac,1000,3600",
      errors: {
        channel: "",
        text: "",
      }
    };
  }

  /**
   * Convert PhantomBot CSV to JSON.
   *
   * @param {string} text the text to convert.
   */
  convertJson(text) {
    var json = [];

    if (!this.state.channel) {
      this.setState({
        errors: {
          channel: "Channel must be specified",
        },
      });

      throw new Error("Channel must be specified");
    }

    let document = importCsv(text);

    for (var line of document) {
      var user = line["Name"]
      var amount = parseInt(line["Balance"].trim());
      var watch_time = parseInt(line["TimeInSeconds"].trim());

      json.push({
        channel: this.state.channel,
        user,
        amount,
        watch_time,
      });
    }

    return json;
  }

  /**
   * Import PhantomBot CSV to OxidizeBot.
   *
   * @param {*} e the event being handled.
   */
  async import(e) {
    this.setState({
      errors: {}
    });

    e.preventDefault();
    var json = [];

    try {
      json = this.convertJson(this.state.text);
    } catch(e) {
      console.log(e);
      return;
    }

    this.setState({
      loading: true,
    });

    try {
      await this.api.importBalances(json);

      this.setState({
        loading: false,
        error: null,
        success: "Successfully imported balances!",
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `Failed to import balances: ${e}`,
        success: null,
      });
    }
  }

  handleChannelChange(e) {
    this.setState({
      channel: e.target.value
    });
  }

  handleTextChange(e) {
    this.setState({
      text: e.target.value
    });
  }

  render() {
    var message = null;

    if (!!this.state.success) {
      message = <Alert variant="info">{this.state.success}</Alert>;
    }

    if (!!this.state.error) {
      message = <Alert variant="danger">{this.state.error}</Alert>;
    }

    var channelError = null;

    if (!!this.state.errors.channel) {
      channelError = <Form.Control.Feedback type="invalid">{this.state.errors.channel}</Form.Control.Feedback>;
    }

    var textError = null;

    if (!!this.state.errors.text) {
      textError = <Form.Control.Feedback type="invalid">{this.state.errors.text}</Form.Control.Feedback>;
    }

    return (
      <div>
      {message}
      <Form onSubmit={e => this.import(e)} disabled={this.state.loading}>
        <Form.Group id="channel">
          <Form.Label>Channel</Form.Label>
          <Form.Control
            disabled={this.state.loading}
            isInvalid={!!this.state.errors.channel}
            value={this.state.channel}
            onChange={e => this.handleChannelChange(e)}
            placeholder="#setbac" />
          {channelError}
          <Form.Text>
            Name of channel to import balances for. Like <b>#setbac</b>.
          </Form.Text>
        </Form.Group>

        <Form.Group id="content">
          <Form.Control as="textarea" rows="10"
            disabled={this.state.loading}
            isInvalid={!!this.state.errors.text}
            value={this.state.text}
            onChange={e => this.handleTextChange(e)} />
          {textError}
          <Form.Text>
            Balances to import. Each line should be <code>name,balance,watch_time</code>.
          </Form.Text>
        </Form.Group>

        <Button variant="primary" type="submit" disabled={this.state.loading}>
          Import
        </Button>

        <Loading isLoading={this.state.loading} />
      </Form>
      </div>
    );
  }
}


class PhantomBotImportCsvForm extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      success: null,
      error: null,
      channel: "",
      text: "",
      errors: {
        channel: "",
        text: "",
      }
    };
  }

  /**
   * Convert PhantomBot CSV to JSON.
   *
   * @param {string} text the text to convert.
   */
  convertJson(text) {
    var json = [];

    if (!this.state.channel) {
      this.setState({
        errors: {
          channel: "Channel must be specified",
        },
      });

      throw new Error("Channel must be specified");
    }

    for (var line of text.split("\n")) {
      line = line.trim();

      if (line === "") {
        continue;
      }

      var cols = line.split(",");

      if (cols.length !== 3) {
        this.setState({
          errors: {
            text: `expected 3 columns but got: ${line}`,
          },
        });

        throw new Error(`expected 3 columns but got: ${line}`);
      }

      var user = cols[1].trim();
      var amountText = cols[2].trim();

      if (amountText === "null") {
        continue;
      }

      var amount = 0;

      try {
        amount = parseInt(amountText);
      } catch {
        throw new Error(`expected numeric third column on line: ${line}`);
      }

      json.push({
        channel: this.state.channel,
        user,
        amount,
      });
    }

    return json;
  }

  /**
   * Import PhantomBot CSV to OxidizeBot.
   *
   * @param {*} e the event being handled.
   */
  async import(e) {
    this.setState({
      errors: {}
    });

    e.preventDefault();
    var json = [];

    try {
      json = this.convertJson(this.state.text);
    } catch(e) {
      console.log(e);
      return;
    }

    this.setState({
      loading: true,
    });

    try {
      await this.api.importBalances(json);

      this.setState({
        loading: false,
        error: null,
        success: "Successfully imported balances!",
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `Failed to import balances: ${e}`,
        success: null,
      });
    }
  }

  handleChannelChange(e) {
    this.setState({
      channel: e.target.value
    });
  }

  handleTextChange(e) {
    this.setState({
      text: e.target.value
    });
  }

  render() {
    var message = null;

    if (!!this.state.success) {
      message = <Alert variant="info">{this.state.success}</Alert>;
    }

    if (!!this.state.error) {
      message = <Alert variant="danger">{this.state.error}</Alert>;
    }

    var channelError = null;

    if (!!this.state.errors.channel) {
      channelError = <Form.Control.Feedback type="invalid">{this.state.errors.channel}</Form.Control.Feedback>;
    }

    var textError = null;

    if (!!this.state.errors.text) {
      textError = <Form.Control.Feedback type="invalid">{this.state.errors.text}</Form.Control.Feedback>;
    }

    return (
      <div>
      {message}
      <Form onSubmit={e => this.import(e)} disabled={this.state.loading}>
        <Form.Group id="channel">
          <Form.Label>Channel</Form.Label>
          <Form.Control
            disabled={this.state.loading}
            isInvalid={!!this.state.errors.channel}
            value={this.state.channel}
            onChange={e => this.handleChannelChange(e)}
            placeholder="#setbac" />
          {channelError}
          <Form.Text>
            Name of channel to import balances for. Like <b>#setbac</b>.
          </Form.Text>
        </Form.Group>

        <Form.Group id="content">
          <Form.Control as="textarea" rows="10"
            disabled={this.state.loading}
            isInvalid={!!this.state.errors.text}
            value={this.state.text}
            onChange={e => this.handleTextChange(e)}
            placeholder=",PhantomBot,1000" />
          {textError}
          <Form.Text>
            Balances to import. Each line should be <code>,user,amount</code>.
          </Form.Text>
        </Form.Group>

        <Button variant="primary" type="submit" disabled={this.state.loading}>
          Import
        </Button>

        <Loading isLoading={this.state.loading} />
      </Form>
      </div>
    );
  }
}

function importCsv(text) {
  let out = [];
  let lines = text.split('\n');

  let columnNames = lines[0].split(',');

  for (line of lines.slice(1)) {
    let cols = line.split(',');
    let line = {};

    for (let i = 0; i < columnNames.length; i++) {
      line[columnNames[i]] = cols[i];
    }

    out.push(line);
  }

  return out;
}