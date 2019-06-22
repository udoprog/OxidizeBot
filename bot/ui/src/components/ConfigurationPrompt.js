import React from "react";
import {Row, Col, Alert} from "react-bootstrap";
import Settings from "./Settings.js";

export default class ConfigurationPrompt extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      configured: true,
      loading: false,
      error: null,
    }
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list();
  }

  list() {
    this.setState({
      loading: true,
    });

    if (this.props.hideWhenConfigured) {
      this.props.api.settings(this.props.filter)
        .then(settings => {
          this.setState({
            configured: settings.every(s => s.value !== null),
            loading: false,
          })
        },
        e => {
          this.setState({
            error: e,
            loading: false,
          })
        });
      }
  }

  render() {
    if (this.props.hideWhenConfigured && this.state.configured) {
      return null;
    }

    let error = null;

    if (this.state.error) {
      error = <Alert key="error" variant="warning">{this.state.error}</Alert>;
    }

    let help = null;

    if (this.props.children) {
      help = (
        <Row key="help">
          <Col>
            {this.props.children}
          </Col>
        </Row>
      );
    }

    return [
      error,
      help,
      <Row key="settings">
        <Col>
          <Settings
            useTitle={this.props.useTitle}
            disableDoc={this.props.disableDoc}
            group={this.props.group}
            api={this.props.api}
            filter={this.props.filter}
            filterable={!!this.props.filterable}
            location={this.props.location}
            history={this.props.history} />
        </Col>
      </Row>,
    ];
  }
}