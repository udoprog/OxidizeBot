import React from "react";
import { Table, Alert, Spinner } from "react-bootstrap";
import { Link } from "react-router-dom";
import { api } from "../globals.js";
import { RouteLayout } from "./Layout";
import Loading from "./Loading";

export default class Players extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      loading: true,
      players: []
    };
  }

  async componentDidMount() {
    let players = await api.players();
    this.setState({players, loading: false});
  }

  render() {
    let table = null;

    if (!this.state.loading) {
      if (this.state.players.length === 0) {
        table = (
          <Alert variant="primary">
            There are currently no players!
          </Alert>
        );
      } else {
        table = (
          <Table striped bordered hover>
            <tbody>
              {this.state.players.map(p => {
                return (
                  <tr key={p}>
                    <td><Link to={`/player/${p}`}>{p}</Link></td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <RouteLayout>
        <h2 className="page-title">Players</h2>
        <Loading isLoading={this.state.loading} />
        {table}
      </RouteLayout>
    );
  }
}