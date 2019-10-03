import {Spinner} from "../utils.js";
import React from "react";
import {Alert, Table} from "react-bootstrap";

export default class Connections extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: true,
      error: null,
      connections: [],
    };
  }

  async componentDidMount() {
    try {
      let connections = await this.api.listConnections();

      this.setState({
        loading: false,
        error: null,
        connections,
      });
    } catch (e) {
      this.setState({
        loading: false,
        error: `failed to request connections: ${e}`,
      });
    }
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let content = null;

    if (this.state.loading) {
      content = <Spinner />;
    } else {
      content = (
        <Table responsive="sm">
          <tbody>
            {this.state.connections.map((c, id) => {
              return (
                <tr key={id}>
                  <td>
                    <b>{c.title}</b><br />
                    {c.description}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </Table>
      );
    }

    return (
      <>
        <p>
          These are your active connections.
          You can manage them in <a href="https://setbac.tv/connections">My Connections on setbac.tv</a>.
        </p>

        {error}
        {content}
      </>
    );
  }
}