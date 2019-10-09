import React from "react";
import Settings from "./Settings.js";

export default class ConfigurationPrompt extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      configured: true,
    }

    this.onLoading = () => {};

    if (this.props.onLoading !== undefined) {
      this.onLoading = this.props.onLoading;
    }

    this.onError = () => {};

    if (this.props.onError !== undefined) {
      this.onError = this.props.onError;
    }
  }

  async componentDidMount() {
    await this.list();
  }

  async list() {
    if (!this.props.hideWhenConfigured) {
      return;
    }

    this.onLoading(true);

    try {
      let settings = await this.props.api.settings(this.props.filter);

      this.onLoading(false);

      this.setState({
        configured: settings.every(s => s.value !== null),
      });
    } catch(e) {
      this.onError(e);
    }
  }

  render() {
    if (this.props.hideWhenConfigured && this.state.configured) {
      return null;
    }

    return <>
      {this.props.children}
      <Settings
        useTitle={this.props.useTitle}
        disableDoc={this.props.disableDoc}
        group={this.props.group}
        api={this.props.api}
        filter={this.props.filter}
        filterable={!!this.props.filterable}
        location={this.props.location}
        history={this.props.history}
        onLoading={this.onLoading}
        onError={this.onError}
        />
    </>;
  }
}