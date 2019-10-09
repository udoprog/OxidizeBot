import React from "react";
import {Row, Col, Alert} from "react-bootstrap";
import Settings from "./Settings.js";
import Error from 'shared-ui/components/Error';

export default class ConfigurationPrompt extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      configured: true,
      loading: false,
      error: null,
    }
  }

  async componentDidMount() {
    await this.list();
  }

  async list() {
    if (!this.props.hideWhenConfigured) {
      return;
    }

    // test if configured...
    this.setState({loading: true});

    try {
      let settings = await this.props.api.settings(this.props.filter);

      this.setState({
        configured: settings.every(s => s.value !== null),
        loading: false,
      });
    } catch(e) {
      this.setState({
        error: e,
        loading: false,
      });
    }
  }

  render() {
    if (this.props.hideWhenConfigured && this.state.configured) {
      return null;
    }

    return <>
      <Error error={this.state.error} />
      {this.props.children}
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
      </Row>
    </>;
  }
}