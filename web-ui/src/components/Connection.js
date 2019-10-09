import React from "react";
import { Button, ButtonGroup } from "react-bootstrap";
import { api, currentUser } from "../globals.js";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import copy from 'copy-to-clipboard';

export default class Connection extends React.Component {
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

    return <div className="oxi-connected-meta">Connected account: {account}</div>;
  }

  validate() {
    if (this.props.outdated) {
      return <div className="oxi-connected-validate danger">
        <b>Connection is outdated, in order to use it properly it needs to be refreshed!</b>
      </div>;
    }

    switch (this.props.type) {
      case "spotify":
        if (!this.props.meta || !this.props.meta.product) {
          return null;
        }

        if (this.props.meta.product === "premium") {
          return null;
        }

        return <div className="oxi-connected-validate danger"><b>You need a Premium Spotify Account</b></div>;
      default:
        return null;
    }
  }

  render() {
    let icon = this.icon();
    let buttons = [];
    let button = null;

    if (this.props.connected !== null) {
      let copy = false;

      if (!this.props.connected) {
        buttons.push(
          <Button key="connect" disabled={currentUser === null} size="sm" variant="primary" onClick={() => this.connect()} title="Connect">
            <FontAwesomeIcon icon="plug" />
          </Button>
        );

        copy = true;
      }

      if (this.props.outdated) {
        buttons.push(
          <Button key="refresh" size="sm" variant="warning" onClick={() => this.connect()} title="Connection is outdated and need to be refreshed!">
            <FontAwesomeIcon icon="sync" />
          </Button>
        );

        copy = true;
      }

      if (copy) {
        if (this.state.copied) {
          buttons.push(
            <Button key="copy" size="sm" variant="success" disabled={true}>
              <FontAwesomeIcon icon="check" />
            </Button>
          );
        } else {
          buttons.push(
            <Button key="copy" disabled={currentUser === null} size="sm" variant="success" onClick={() => this.copy()} title="Copy connection URL to clipboard">
              <FontAwesomeIcon icon="copy" />
            </Button>
          );
        }
      }

      if (this.props.connected) {
        buttons.push(
          <Button key="remove" size="sm" variant="danger" onClick={() => this.disconnect()} title="Remove connection">
            <FontAwesomeIcon icon="trash" />
          </Button>
        );
      } else {
        buttons.push(
          <Button disabled={true} key="remove" size="sm" variant="light" title="Connection not present">
            <FontAwesomeIcon icon="trash" />
          </Button>
        );
      }
    }

    buttons = <ButtonGroup>{buttons}</ButtonGroup>;

    let meta = this.meta();
    let validate = this.validate();

    return (
      <tr>
        <td className="oxi-connected">
          <div className="oxi-connected-title">{icon} {this.props.title}</div>
          {meta}
          {validate}
          <div className="oxi-connected-description">{this.props.description}</div>
        </td>
        <td align="right">{buttons}</td>
      </tr>
    );
  }
}