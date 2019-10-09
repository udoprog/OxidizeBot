import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import ConfigurationPrompt from "./ConfigurationPrompt";
import {Loading, Error} from 'shared-ui/components';

export default class Promotions extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
    };
  }

  async componentDidMount() {
    await this.list();
  }

  /**
   * Refresh the list of after streams.
   */
  async list() {
    this.setState({
      loading: true,
    });

    try {
      let data = await this.api.promotions(this.props.current.channel);

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to request after streams: ${e}`,
        data: null,
      });
    }
  }

  async editDisabled(key, disabled) {
    this.setState({
      loading: true,
      error: null,
    });

    try {
      await this.api.promotionsEditDisabled(key, disabled);
      await this.list();
    } catch(e) {
      this.setState({
        loading: false,
        error: `Failed to set disabled state: ${e}`,
      });
    }
  }

  render() {
    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No promotions!
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
                    <td className="promotion-name">{c.key.name}</td>
                    <td className="promotion-group"><b>{c.group}</b></td>
                    <td className="promotion-template">{c.template}</td>
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
      <h1 className="oxi-page-title">Promotions</h1>
      <Loading isLoading={this.state.loading} />
      <Error error={this.state.error} />
      <ConfigurationPrompt api={this.api} filter={{prefix: ["promotions"]}} />
      {content}
    </>;
  }
}