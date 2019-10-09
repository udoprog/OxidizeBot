import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import Loading from 'shared-ui/components/Loading';

export default class Aliases extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: true,
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
      let data = await this.api.aliases(this.props.current.channel);

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch (e) {
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
      await this.api.aliasesEditDisabled(key, disabled);
    } catch(e) {
      this.setState({
        loading: false,
        error: `Failed to set disabled state: ${e}`,
      });
    }

    return this.list();
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No aliases!
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
                    <td className="alias-name">{c.key.name}</td>
                    <td className="alias-group"><b>{c.group}</b></td>
                    <td className="alias-template">{c.template}</td>
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
      <h1 className="oxi-page-title">Aliases</h1>
      <Loading isLoading={this.state.loading} />
      {error}
      {content}
    </>;
  }
}