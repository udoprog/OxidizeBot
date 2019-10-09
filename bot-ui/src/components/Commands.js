import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import ConfigurationPrompt from "./ConfigurationPrompt";
import { Loading, Error } from 'shared-ui/components';

export default class Commands extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      configLoading: false,
      error: null,
      data: null,
    };
  }

  async componentDidMount() {
    await this.list()
  }

  /**
   * Refresh the list of after streams.
   */
  async list() {
    this.setState({
      loading: true,
    });

    try {
      let data = await this.api.commands(this.props.current.channel);

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to request after streams: ${e}`,
      });
    }
  }

  async editDisabled(key, disabled) {
    this.setState({
      loading: true,
      error: null,
    });

    await this.api.commandsEditDisabled(key, disabled);
    await this.list();

    try {
      this.setState({
        loading: false,
        error: `Failed to set disabled state: ${e}`,
      });
    } catch (e) {
      e => {
        this.setState({
          loading: false,
          error: `Failed to set disabled state: ${e}`,
        });
      }
    }
  }

  render() {
    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No commands!
          </Alert>
        );
      } else {
        content = (
          <Table responsive="sm">
            <thead>
              <tr>
                <th>Name</th>
                <th>Group</th>
                <th className="table-fill">Text</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.map((c, id) => {
                let disabled = null;

                if (c.disabled) {
                  let onClick = _ => {
                    this.editDisabled(c.key, false);
                  };
                  disabled = <Button className="button-fill" size="sm" variant="danger" onClick={onClick}>Disabled</Button>;
                } else {
                  let onClick = _ => {
                    this.editDisabled(c.key, true);
                  };
                  disabled = <Button className="button-fill" size="sm" variant="success" onClick={onClick}>Enabled</Button>;
                }

                return (
                  <tr key={id}>
                    <td className="command-name">{c.key.name}</td>
                    <td className="command-group"><b>{c.group}</b></td>
                    <td className="command-template">{c.template}</td>
                    <td>{disabled}</td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return <>
      <h1 className="oxi-page-title">Commands</h1>

      <Error error={this.state.error} />
      <Loading isLoading={this.state.loading || this.state.configLoading} />

      <ConfigurationPrompt
        api={this.api} filter={{prefix: ["command"]}}
        onLoading={configLoading => this.setState({configLoading, error: null})}
        onError={error => this.setState({configLoading: false, error})}
      />

      {content}
    </>;
  }
}