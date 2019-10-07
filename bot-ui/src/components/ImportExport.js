import React from "react";
import {Button, Form, Alert} from "react-bootstrap";
import { faUtensilSpoon } from "@fortawesome/fontawesome-free-solid";
import * as utils from "../utils.js";

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
  import(e) {
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

    this.api.importBalances(json).then(() => {
      this.setState({
        loading: false,
        error: null,
        success: "Successfully imported balances!",
      });
    }, e => {
      this.setState({
        loading: false,
        error: `Failed to import balances: ${e}`,
        success: null,
      });
    });
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

    var spinner = null;

    if (this.state.loading) {
      spinner = <utils.Spinner />;
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

        {spinner}
      </Form>
      </div>
    );
  }
}

export default class ImportExport extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
    };
  }

  exportPhantomBotCsv() {
    this.api.exportBalances().then(balances => {
      var balancesCsv = "";

      for (var balance of balances) {
        balancesCsv += `,${balance.user},${balance.amount}\r\n`;
      }

      utils.download("text/plain", balancesCsv, "balances.csv");
    });
  }

  render() {
    return (
      <div>
        <h4>
          PhantomBot CSV Export
        </h4>

        <Form onSubmit={() => this.exportPhantomBotCsv()}>
          <Button type="submit">Export to File</Button>
        </Form>

        <h4>
          PhantomBot CSV Import
        </h4>

        <PhantomBotImportCsvForm api={this.api} />
      </div>
    );
  }
}